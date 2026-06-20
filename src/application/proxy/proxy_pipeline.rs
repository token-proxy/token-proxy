//! 代理转发应用服务 — application/proxy/
//!
//! 编排一次 LLM API 代理转发的完整流程。
//! 领域逻辑委托给 `AccessPointEx`、`FaultService`、`RoutingStrategy` 等，
//! 本层仅负责加载数据、调用领域行为、转发 HTTP、构造响应。

use std::sync::Arc;
use std::time::Instant;

use axum::http::HeaderMap;
use bytes::Bytes;
use futures::StreamExt;
use uuid::Uuid;

use crate::application::log::dto::ProxyLogData;
use crate::application::log::LogService;
use crate::domain::access_point::repository::AccessPointRepository;
use crate::domain::access_point::SessionAffinityRepository;
use crate::domain::provider::repository::AccountRepository;
use crate::domain::provider::repository::ProviderRepository;
use crate::domain::provider::FaultService;
use crate::domain::provider::FaultOutcome;
use crate::domain::shared::EncryptionService;
use crate::domain::shared::HOP_BY_HOP_HEADERS;
use crate::infrastructure::http_client::ProcessedRequest;
use crate::infrastructure::http_client::ProxyClient;
use crate::infrastructure::http_client::ProxyLogger;
use crate::shared::error::AppError;

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
}

impl ProxyPipeline {
    pub fn new(
        access_point_repo: Arc<dyn AccessPointRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        account_repo: Arc<dyn AccountRepository>,
        encryption_service: Arc<dyn EncryptionService>,
        proxy_client: Arc<ProxyClient>,
        log_service: Arc<LogService>,
        session_affinity_repo: Arc<dyn SessionAffinityRepository>,
    ) -> Self {
        ProxyPipeline {
            access_point_repo,
            provider_repo,
            account_repo,
            encryption_service,
            proxy_client,
            log_service,
            session_affinity_repo,
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

        let session_id = headers
            .get("x-claude-code-session-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        if session_id != "unknown" {
            if let Some(affinity) = self
                .session_affinity_repo
                .find_by_access_point_and_session(access_point.id, &session_id)
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
                .ok_or_else(|| {
                    AppError::Internal("后端关联的 Provider 未找到".to_string())
                })?;

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
            let upstream_key =
                String::from_utf8(decrypted)
                    .map_err(|_| AppError::Internal("API Key 解码失败".to_string()))?;

            // 4d. 构造上游请求
            let processed = ProcessedRequest::prepare(
                &access_point,
                &upstream_key,
                remainder,
                headers.clone(),
                &body,
                &provider,
            )?;

            let body_bytes =
                Bytes::from(serde_json::to_string(processed.outbound.body()).unwrap_or_default());
            let start = Instant::now();

            // 4e. 转发到上游
            let upstream_resp = self
                .proxy_client
                .forward(
                    &processed.upstream_url,
                    processed.outbound.headers().clone(),
                    body_bytes,
                )
                .await?;

            let status = upstream_resp.status();
            let resp_headers = upstream_resp.headers().clone();
            let is_sse = resp_headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.contains("text/event-stream"))
                .unwrap_or(false);

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
                let log_data = build_proxy_log_data(
                    &processed,
                    &access_point,
                    user_id,
                    status.as_u16(),
                    resp_headers.clone(),
                    provider.id,
                    account_entry.account_id,
                );
                let logger = ProxyLogger::new(log_data, self.log_service.clone(), start);

                if is_sse {
                    let byte_stream = upstream_resp.bytes_stream();
                    let stream = async_stream::stream! {
                        tokio::pin!(byte_stream);
                        let mut logger = logger;
                        while let Some(chunk) = byte_stream.next().await {
                            match chunk {
                                Ok(bytes) => {
                                    logger.append_body(&bytes);
                                    yield Ok(bytes);
                                }
                                Err(e) => {
                                    yield Err(axum::Error::new(e));
                                }
                            }
                        }
                        logger.flush();
                    };

                    // 保存会话粘滞（异步，不阻塞响应）
                    spawn_save_affinity(
                        self.session_affinity_repo.clone(),
                        access_point.id,
                        &session_id,
                        account_entry.account_id,
                    );

                    return response_builder
                        .body(axum::body::Body::from_stream(stream))
                        .map_err(|e| AppError::Internal(format!("构建响应失败: {}", e)));
                } else {
                    let resp_body = upstream_resp
                        .bytes()
                        .await
                        .map_err(|e| AppError::Upstream(format!("读取上游响应失败: {}", e)))?;

                    let mut logger = logger;
                    logger.set_body(&resp_body);
                    logger.flush();

                    spawn_save_affinity(
                        self.session_affinity_repo.clone(),
                        access_point.id,
                        &session_id,
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
                    let fault = FaultService::detect(
                        status_u16,
                        provider.rate_limit_config.as_ref(),
                        provider.balance_exhausted_config.as_ref(),
                        &resp_headers,
                        &[],
                    );

                    let log_data = build_proxy_log_data(
                        &processed,
                        &access_point,
                        user_id,
                        status_u16,
                        resp_headers.clone(),
                        provider.id,
                        account_entry.account_id,
                    );
                    let logger = ProxyLogger::new(log_data, self.log_service.clone(), start);

                    let byte_stream = upstream_resp.bytes_stream();
                    let stream = async_stream::stream! {
                        tokio::pin!(byte_stream);
                        let mut logger = logger;
                        while let Some(chunk) = byte_stream.next().await {
                            match chunk {
                                Ok(bytes) => {
                                    logger.append_body(&bytes);
                                    yield Ok(bytes);
                                }
                                Err(e) => {
                                    yield Err(axum::Error::new(e));
                                }
                            }
                        }
                        logger.flush();
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
                let resp_body = upstream_resp
                    .bytes()
                    .await
                    .map_err(|e| AppError::Upstream(format!("读取上游响应失败: {}", e)))?;

                // 始终记录日志
                let log_data = build_proxy_log_data(
                    &processed,
                    &access_point,
                    user_id,
                    status_u16,
                    resp_headers.clone(),
                    provider.id,
                    account_entry.account_id,
                );
                let mut logger = ProxyLogger::new(log_data, self.log_service.clone(), start);
                logger.set_body(&resp_body);
                logger.flush();

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

// ─── 辅助函数 ────────────────────────────────────────────────────────

/// 从已处理的请求和接入点信息构造 `ProxyLogData`
fn build_proxy_log_data(
    processed: &ProcessedRequest,
    access_point: &crate::domain::access_point::AccessPointEx,
    user_id: Uuid,
    status_code: u16,
    resp_headers: HeaderMap,
    provider_id: Uuid,
    account_id: Uuid,
) -> ProxyLogData {
    ProxyLogData {
        timestamp: chrono::Utc::now().fixed_offset(),
        session_id: processed.session_id.clone(),
        user_id,
        access_point_id: access_point.id,
        provider_id,
        account_id,
        model_original: processed.inbound.model().to_string(),
        model_mapped: processed.outbound.model().to_string(),
        api_type: access_point.api_type.to_string(),
        status_code,
        request_headers: processed.inbound.headers().clone(),
        request_body: processed.inbound.body().clone(),
        resp_headers,
        response_body: String::new(),
        duration_ms: 0,
        is_interrupted: false,
        error_message: None,
    }
}

/// 异步保存会话粘滞绑定（spawn 到后台，不阻塞响应返回）
fn spawn_save_affinity(
    repo: Arc<dyn SessionAffinityRepository>,
    access_point_id: Uuid,
    session_id: &str,
    account_id: Uuid,
) {
    if session_id == "unknown" {
        return;
    }
    let sid = session_id.to_string();
    tokio::spawn(async move {
        if let Err(e) = repo.upsert(access_point_id, &sid, account_id).await {
            tracing::warn!(error = %e, "会话绑定保存失败");
        }
    });
}
