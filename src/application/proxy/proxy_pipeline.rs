//! 代理转发应用服务 — application/proxy/
//!
//! 编排一次 LLM API 代理转发的完整流程。
//! 领域逻辑委托给 `AccessPointEx`、`FaultService`、`UpstreamOutcome` 等，
//! 本层仅负责加载数据、调用领域行为、转发 HTTP、构造响应。
//!
//! `execute` 是调度骨架（~60 行），`try_one_account` 处理单个候选账号的完整尝试。

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::http::HeaderMap;
use bytes::Bytes;
use uuid::Uuid;

use crate::application::log::LogService;
use crate::domain::access_point::repository::AccessPointRepository;
use crate::domain::access_point::SessionAffinityRepository;
use crate::domain::provider::repository::AccountRepository;
use crate::domain::provider::repository::ProviderRepository;
use crate::domain::provider::FaultService;
use crate::domain::proxy::{RetryDecision, UpstreamOutcome};
use crate::domain::shared::{EncryptionService, InboundRequest};
use crate::infrastructure::http_client::ProxyClient;
use crate::shared::error::AppError;

use super::account_selector::{AccountCandidate, AccountSelector};
use super::proxy_call_record::ProxyCallRecord;
use super::response_builder::{build_buffered_response, build_streaming_response};
use super::tracked_spawner::TrackedSpawner;
use super::upstream_dispatcher::UpstreamDispatcher;

/// 代理转发管道
///
/// 编排一次 LLM API 代理转发的完整流程：
/// 聚合加载 → 账户排序与粘滞 → 候选迭代（解密 + 转发 + 分类 + 决策）→ 日志记录
pub struct ProxyPipeline {
    access_point_repo: Arc<dyn AccessPointRepository>,
    account_repo: Arc<dyn AccountRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    encryption_service: Arc<dyn EncryptionService>,
    dispatcher: UpstreamDispatcher,
    log_service: Arc<LogService>,
    session_affinity_repo: Arc<dyn SessionAffinityRepository>,
    spawner: TrackedSpawner,
    shutting_down: Arc<AtomicBool>,
}

impl ProxyPipeline {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        access_point_repo: Arc<dyn AccessPointRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        account_repo: Arc<dyn AccountRepository>,
        encryption_service: Arc<dyn EncryptionService>,
        proxy_client: Arc<ProxyClient>,
        log_service: Arc<LogService>,
        session_affinity_repo: Arc<dyn SessionAffinityRepository>,
        spawner: TrackedSpawner,
        shutting_down: Arc<AtomicBool>,
    ) -> Self {
        ProxyPipeline {
            access_point_repo,
            account_repo,
            provider_repo,
            encryption_service,
            dispatcher: UpstreamDispatcher::new(proxy_client),
            log_service,
            session_affinity_repo,
            spawner,
            shutting_down,
        }
    }

    /// 执行一次代理转发 — 调度骨架
    pub async fn execute(
        &self,
        short_code: &str,
        remainder: &str,
        headers: HeaderMap,
        body: String,
        user_id: Uuid,
    ) -> Result<axum::response::Response, AppError> {
        // ── 0. 优雅关闭短路 ──
        if self.shutting_down.load(Ordering::Acquire) {
            return Err(AppError::Upstream("服务正在关闭，请稍后重试".to_string()));
        }

        // ── 1. 加载并准备接入点聚合根 ──
        let mut access_point = self
            .access_point_repo
            .find_by_short_code(short_code)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("接入点 '{}' 未找到", short_code)))?;
        access_point.validate_usable()?;
        if !access_point.has_available_accounts() {
            return Err(AppError::Forbidden("接入点无可用账号".to_string()));
        }

        // ── 2. 协议解析入站请求 + 提取会话标识 ──
        let inbound = access_point.api_type.parse_inbound(headers, body)?;
        let session_id = access_point.api_type.extract_session_id(&inbound);

        // ── 3. 账号排序（按路由策略）+ 会话粘滞 ──
        access_point.sort_accounts();
        if let Some(sid) = &session_id {
            if let Some(affinity) = self
                .session_affinity_repo
                .find_by_access_point_and_session(access_point.id, sid)
                .await?
            {
                access_point.apply_session_affinity(affinity.account_id);
            }
        }

        // ── 4. 重试循环 ──
        let mut last_error = None;
        let mut selector = AccountSelector::new(
            &access_point,
            self.account_repo.clone(),
            self.provider_repo.clone(),
            self.encryption_service.clone(),
        );
        while let Some(candidate) = selector.next().await? {
            match self
                .try_one_account(
                    &access_point,
                    &inbound,
                    session_id.as_deref(),
                    user_id,
                    candidate,
                    remainder,
                )
                .await?
            {
                RetryDecision::Return(resp) => return Ok(resp),
                RetryDecision::Continue(reason) => {
                    last_error = Some(reason);
                }
            }
        }
        Err(last_error.unwrap_or(AppError::Upstream("所有账号不可用".to_string())))
    }

    /// 尝试一个候选账号：构造上游请求 → 转发 → 分类 → 决策
    ///
    /// 返回值的两个分支由类型系统强制完整性：
    /// - `RetryDecision::Return` — 终止重试，把响应交给客户端
    /// - `RetryDecision::Continue` — 禁用当前账号后切换下一个候选
    async fn try_one_account(
        &self,
        access_point: &crate::domain::access_point::AccessPointEx,
        inbound: &InboundRequest,
        session_id: Option<&str>,
        user_id: Uuid,
        candidate: AccountCandidate,
        remainder: &str,
    ) -> Result<RetryDecision, AppError> {
        let AccountCandidate {
            account,
            account_id,
            provider,
            upstream_key,
        } = candidate;

        // 构造上游请求（领域行为：URL 拼接 + 模型路由 + 协议适配）
        let upstream =
            access_point.build_upstream_request(inbound, &provider, &upstream_key, remainder)?;

        // 启动调用记录器（请求侧已知，启动计时）
        let mut record = ProxyCallRecord::start(
            inbound,
            &upstream,
            access_point,
            session_id.map(|s| s.to_string()),
            user_id,
            provider.id,
            account_id,
            self.log_service.clone(),
            self.spawner.clone(),
        );

        // 转发到上游
        let upstream_resp = self.dispatcher.forward(&upstream).await?;
        let status = upstream_resp.status();
        let resp_headers = upstream_resp.headers().clone();
        let is_sse = access_point.api_type.is_sse_response(&resp_headers);
        record.attach_response(status, &resp_headers);

        // SSE 路径：分类时 body 未读，传 None；后续在 stream 中边消费边累积日志
        // Buffered 路径：先读完整 body，再分类，故障检测可使用 body
        if is_sse {
            let outcome = UpstreamOutcome::classify(&provider, status, &resp_headers, None, true);
            // 故障：禁用账号（流已经开始，不可切换）
            if let UpstreamOutcome::Fault {
                reason,
                available_at,
                ..
            } = &outcome
            {
                self.disable_account_on_fault(
                    status.as_u16(),
                    &account,
                    account_id,
                    reason.clone(),
                    *available_at,
                )
                .await;
            }
            // SSE 成功且未故障：保存会话粘滞
            if matches!(outcome, UpstreamOutcome::Success { .. }) {
                self.save_affinity(access_point.id, session_id, account_id);
            }
            let resp = build_streaming_response(status, &resp_headers, upstream_resp, record)?;
            return Ok(RetryDecision::Return(resp));
        }

        // 非 SSE：读取完整响应体，分类时 body 可用
        let resp_body = UpstreamDispatcher::read_buffered_body(upstream_resp).await?;
        let outcome =
            UpstreamOutcome::classify(&provider, status, &resp_headers, Some(&resp_body), false);

        match outcome {
            UpstreamOutcome::Success { .. } => {
                self.save_affinity(access_point.id, session_id, account_id);
                let resp = build_buffered_response(status, &resp_headers, resp_body, record)?;
                Ok(RetryDecision::Return(resp))
            }
            UpstreamOutcome::ClientError { .. } | UpstreamOutcome::ServerError { .. } => {
                // 透传给客户端，不禁用账号，不重试
                let resp = build_buffered_response(status, &resp_headers, resp_body, record)?;
                Ok(RetryDecision::Return(resp))
            }
            UpstreamOutcome::Fault {
                reason,
                available_at,
                ..
            } => {
                // 禁用账号后切换下一个候选
                let status_u16 = status.as_u16();
                // 让 record 先 finish 落库（包含错误响应体）
                build_buffered_response_finish_only(resp_body, record);
                self.disable_account_on_fault(
                    status_u16,
                    &account,
                    account_id,
                    reason,
                    available_at,
                )
                .await;
                Ok(RetryDecision::Continue(AppError::Upstream(format!(
                    "上游返回 {}，账户已禁用，切换账号",
                    status_u16
                ))))
            }
        }
    }

    /// 异步保存会话粘滞绑定
    fn save_affinity(&self, access_point_id: Uuid, session_id: Option<&str>, account_id: Uuid) {
        let Some(sid) = session_id else {
            return;
        };
        let sid = sid.to_string();
        let repo = self.session_affinity_repo.clone();
        self.spawner.spawn("session_affinity", async move {
            repo.upsert(access_point_id, &sid, account_id)
                .await
                .map(|_| ())
        });
    }

    /// 故障账户禁用和持久化
    ///
    /// 克隆账户并调用 `FaultService::disable_account()`，然后保存到仓库。
    async fn disable_account_on_fault(
        &self,
        status: u16,
        account: &crate::domain::provider::Account,
        account_id: Uuid,
        reason: crate::domain::provider::DisabledReason,
        available_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    ) {
        let reason_dbg = format!("{:?}", reason);
        let mut disabled_account = account.clone();
        FaultService::disable_account(&mut disabled_account, reason, available_at);

        if let Err(e) = self.account_repo.save(&disabled_account).await {
            tracing::error!(
                account_id = %account_id,
                error = %e,
                "禁用账户失败",
            );
        } else {
            tracing::warn!(
                account_id = %account_id,
                status = status,
                reason = %reason_dbg,
                "账户异常，已自动禁用",
            );
        }
    }
}

/// 故障非 SSE 路径专用：把 record 标记落库但不构造 Response（由调用方决定 Continue）
///
/// 与 `build_buffered_response` 的区别仅在于不返回 Response —— 拆出来避免在
/// `try_one_account` 内重复"set_body + finish"两行。
fn build_buffered_response_finish_only(body: Bytes, mut record: ProxyCallRecord) {
    record.set_body(&body);
    record.finish();
}
