//! 代理转发应用服务 — application/proxy/
//!
//! 编排一次 LLM API 代理转发的完整流程。
//! 领域逻辑委托给 `AccessPointEx`、`FaultService`、`RoutingStrategy` 等，
//! 本层仅负责加载数据、调用领域行为、转发 HTTP、构造响应。

use std::sync::Arc;
use std::time::Duration;

use axum::http::HeaderMap;
use bytes::Bytes;
use futures::StreamExt;
use uuid::Uuid;

use crate::application::log::LogService;
use crate::domain::access_point::repository::AccessPointRepository;
use crate::domain::access_point::SessionAffinityRepository;
use crate::domain::provider::repository::AccountRepository;
use crate::domain::provider::repository::ProviderRepository;
use crate::domain::provider::FaultOutcome;
use crate::domain::provider::FaultService;
use crate::domain::shared::EncryptionService;
use crate::domain::shared::HOP_BY_HOP_HEADERS;
use crate::infrastructure::http_client::ProxyClient;
use crate::shared::error::AppError;

use super::proxy_call_record::ProxyCallRecord;
use super::tracked_spawner::TrackedSpawner;

/// 代理转发管道
///
/// 编排一次 LLM API 代理转发的完整流程：
/// 聚合加载 → 账户排序与粘滞 → 解密 → 请求变换 → 上游转发 → 故障检测 → 日志记录
pub struct ProxyPipeline {
    access_point_repo: Arc<dyn AccessPointRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    account_repo: Arc<dyn AccountRepository>,
    encryption_service: Arc<dyn EncryptionService>,
    proxy_client: Arc<ProxyClient>,
    log_service: Arc<LogService>,
    session_affinity_repo: Arc<dyn SessionAffinityRepository>,
    /// 后台写入任务调度器（代理日志 + 会话粘滞），内置 in_flight 计数追踪
    spawner: TrackedSpawner,
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
    ) -> Self {
        ProxyPipeline {
            access_point_repo,
            provider_repo,
            account_repo,
            encryption_service,
            proxy_client,
            log_service,
            session_affinity_repo,
            spawner,
        }
    }

    /// 执行一次代理转发
    pub async fn execute(
        &self,
        short_code: &str,
        remainder: &str,
        headers: HeaderMap,
        body: String,
        user_id: Uuid,
    ) -> Result<axum::response::Response, AppError> {
        // ── 1. 加载接入点聚合根 ──
        let mut access_point = self
            .access_point_repo
            .find_by_short_code(short_code)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("接入点 '{}' 未找到", short_code)))?;

        // ── 2. 验证可用性 ──
        access_point.validate_usable()?;
        if !access_point.has_available_accounts() {
            return Err(AppError::Forbidden("接入点无可用账号".to_string()));
        }

        // ── 3. 账户排序（按路由策略）+ 会话粘滞 ──
        access_point.sort_accounts();

        // 协议解析入站请求（提取 model 等），随后由协议方法提取会话标识。
        // session_id 为 None 表示请求未携带会话标识（例如非 Claude Code 客户端的直接调用）。
        let inbound = access_point
            .api_type
            .parse_inbound(headers.clone(), body.clone())?;
        let session_id = access_point.api_type.extract_session_id(&inbound);

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
        for account_entry in &access_point.accounts {
            // 4a. 加载账户
            let account = self
                .account_repo
                .find_by_id(account_entry.account_id)
                .await?
                .ok_or_else(|| AppError::Internal("账户池中关联的 Account 未找到".to_string()))?;
            if !account.is_available() {
                continue;
            }

            // 4b. 加载 Provider
            let provider = self
                .provider_repo
                .find_by_id(account.provider_id)
                .await?
                .ok_or_else(|| AppError::Internal("后端关联的 Provider 未找到".to_string()))?;

            // 4c. 解密 API Key
            let encrypted_key = self
                .account_repo
                .get_encrypted_api_key(account_entry.account_id)
                .await?;
            let decrypted = self
                .encryption_service
                .decrypt(&encrypted_key)
                .await
                .map_err(|e| AppError::Encryption(e.to_string()))?;
            let upstream_key = String::from_utf8(decrypted)
                .map_err(|_| AppError::Internal("API Key 解码失败".to_string()))?;

            // 4d. 构造上游请求（聚合根编排：URL 拼接 + 模型路由 + 协议适配）
            let upstream = access_point.build_upstream_request(
                &inbound,
                &provider,
                &upstream_key,
                remainder,
            )?;

            let body_bytes = Bytes::from(serde_json::to_string(&upstream.body).unwrap_or_default());

            // 4e. 启动调用记录器（请求侧已知，启动计时）
            let mut record = ProxyCallRecord::start(
                &inbound,
                &upstream,
                &access_point,
                session_id.clone(),
                user_id,
                provider.id,
                account_entry.account_id,
                self.log_service.clone(),
                self.spawner.clone(),
            );

            // 4f. 转发到上游
            let upstream_resp = self
                .proxy_client
                .forward(&upstream.url, upstream.headers.clone(), body_bytes)
                .await?;

            let status = upstream_resp.status();
            let resp_headers = upstream_resp.headers().clone();
            let is_sse = access_point.api_type.is_sse_response(&resp_headers);

            // 登记响应侧到 record（status_code + 响应头）
            record.attach_response(status, &resp_headers);

            // ── 构造响应基础（过滤 hop-by-hop 头）──
            let mut response_builder = axum::response::Response::builder().status(status);
            for (key, value) in resp_headers.iter() {
                let key_lower = key.as_str().to_lowercase();
                if !HOP_BY_HOP_HEADERS.contains(&key_lower.as_str()) {
                    response_builder = response_builder.header(key, value.clone());
                }
            }

            if status.is_success() {
                // ── 成功路径 ──
                if is_sse {
                    let byte_stream = upstream_resp.bytes_stream();
                    let stream = async_stream::stream! {
                        tokio::pin!(byte_stream);
                        let mut record = record;
                        while let Some(chunk) = byte_stream.next().await {
                            match chunk {
                                Ok(bytes) => {
                                    record.append_body(&bytes);
                                    yield Ok(bytes);
                                }
                                Err(e) => {
                                    yield Err(axum::Error::new(e));
                                }
                            }
                        }
                        record.finish();
                    };

                    // 保存会话粘滞（异步，不阻塞响应）
                    self.save_affinity(
                        access_point.id,
                        session_id.as_deref(),
                        account_entry.account_id,
                    );

                    return response_builder
                        .body(axum::body::Body::from_stream(stream))
                        .map_err(|e| AppError::Internal(format!("构建响应失败: {}", e)));
                } else {
                    // 非流式响应：读取完整响应体（有超时保护，防止永久挂起）
                    let resp_body =
                        tokio::time::timeout(Duration::from_secs(120), upstream_resp.bytes())
                            .await
                            .map_err(|_| AppError::Upstream("读取上游响应超时".to_string()))?
                            .map_err(|e| AppError::Upstream(format!("读取上游响应失败: {}", e)))?;

                    record.set_body(&resp_body);
                    record.finish();

                    self.save_affinity(
                        access_point.id,
                        session_id.as_deref(),
                        account_entry.account_id,
                    );

                    return response_builder
                        .body(axum::body::Body::from(resp_body))
                        .map_err(|e| AppError::Internal(format!("构建响应失败: {}", e)));
                }
            } else {
                // ── 非成功路径 ──
                let status_u16 = status.as_u16();

                if is_sse {
                    // SSE 错误：流式透传 + 日志 + 故障检测
                    // 注意：SSE 错误路径无法预读响应体，故障检测仅基于 status + headers
                    let fault = FaultService::detect(
                        status_u16,
                        provider.rate_limit_config.as_ref(),
                        provider.balance_exhausted_config.as_ref(),
                        &resp_headers,
                        &[],
                    );

                    let byte_stream = upstream_resp.bytes_stream();
                    let stream = async_stream::stream! {
                        tokio::pin!(byte_stream);
                        let mut record = record;
                        while let Some(chunk) = byte_stream.next().await {
                            match chunk {
                                Ok(bytes) => {
                                    record.append_body(&bytes);
                                    yield Ok(bytes);
                                }
                                Err(e) => {
                                    yield Err(axum::Error::new(e));
                                }
                            }
                        }
                        record.finish();
                    };

                    // SSE 故障：如果有故障检测结果，禁用账户
                    if let FaultOutcome::Fault {
                        reason,
                        available_at,
                    } = fault
                    {
                        self.disable_account_on_fault(
                            status_u16,
                            &account,
                            account_entry.account_id,
                            reason,
                            available_at,
                        )
                        .await;
                    }

                    return response_builder
                        .body(axum::body::Body::from_stream(stream))
                        .map_err(|e| AppError::Internal(format!("构建响应失败: {}", e)));
                }

                // 非 SSE 错误：读取完整 body
                // 非流式错误响应：读取完整响应体（有超时保护，防止永久挂起）
                let resp_body =
                    tokio::time::timeout(Duration::from_secs(120), upstream_resp.bytes())
                        .await
                        .map_err(|_| AppError::Upstream("读取上游错误响应超时".to_string()))?
                        .map_err(|e| AppError::Upstream(format!("读取上游响应失败: {}", e)))?;

                // 始终记录日志
                record.set_body(&resp_body);
                record.finish();

                // 故障检测（使用实际响应体）
                let fault = FaultService::detect(
                    status_u16,
                    provider.rate_limit_config.as_ref(),
                    provider.balance_exhausted_config.as_ref(),
                    &resp_headers,
                    &resp_body,
                );

                if let FaultOutcome::Fault {
                    reason,
                    available_at,
                } = fault
                {
                    self.disable_account_on_fault(
                        status_u16,
                        &account,
                        account_entry.account_id,
                        reason,
                        available_at,
                    )
                    .await;

                    last_error = Some(AppError::Upstream(format!(
                        "上游返回 {}，账户已禁用，切换账号",
                        status_u16
                    )));
                    continue;
                }

                // 非账户异常：返回错误响应
                return response_builder
                    .body(axum::body::Body::from(resp_body))
                    .map_err(|e| AppError::Internal(format!("构建响应失败: {}", e)));
            }
        }

        Err(last_error.unwrap_or(AppError::Upstream("所有账号不可用".to_string())))
    }

    /// 异步保存会话粘滞绑定（spawn 到后台，不阻塞响应返回）
    ///
    /// 通过 `TrackedSpawner` 入计数器，优雅关闭时等待落库。
    /// `session_id` 为 `None` 时跳过（请求未携带会话标识）。
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
    /// 在 SSE 和非 SSE 错误路径中复用，消除重复的禁用→保存→日志逻辑。
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
        crate::domain::provider::FaultService::disable_account(
            &mut disabled_account,
            reason,
            available_at,
        );

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
