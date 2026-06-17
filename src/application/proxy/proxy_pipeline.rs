use std::sync::Arc;
use std::time::Instant;

use axum::http::HeaderMap;
use bytes::Bytes;
use futures::StreamExt;
use uuid::Uuid;

use super::acl::log_context::LogContext;
use super::acl::log_task_context::spawn_log_task;
use crate::application::log::LogService;
use crate::domain::access_point::repository::AccessPointRepository;
use crate::domain::access_point::AccessPointEx;
use crate::domain::shared::EncryptionService;
use crate::infrastructure::http_client::ProcessedRequest;
use crate::infrastructure::http_client::ProxyClient;
use crate::shared::error::AppError;

/// 代理转发管道
///
/// 编排一次 LLM API 代理转发的完整流程：
/// 聚合加载 → 解密 → 请求变换 → 上游转发 → 日志记录
pub struct ProxyPipeline {
    access_point_repo: Arc<dyn AccessPointRepository>,
    encryption_service: Arc<dyn EncryptionService>,
    proxy_client: Arc<ProxyClient>,
    log_service: Arc<LogService>,
}

impl ProxyPipeline {
    pub fn new(
        access_point_repo: Arc<dyn AccessPointRepository>,
        encryption_service: Arc<dyn EncryptionService>,
        proxy_client: Arc<ProxyClient>,
        log_service: Arc<LogService>,
    ) -> Self {
        ProxyPipeline {
            access_point_repo,
            encryption_service,
            proxy_client,
            log_service,
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
        let access_point = self
            .access_point_repo
            .find_by_short_code(short_code)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("接入点 '{}' 未找到", short_code)))?;

        access_point.validate_usable()?;

        let upstream_key = access_point.decrypt_upstream_key(&*self.encryption_service).await?;

        let processed = ProcessedRequest::prepare(
            &access_point,
            &upstream_key,
            remainder,
            headers,
            &body,
        )?;

        self.handle(processed, &access_point, user_id).await
    }

    /// 统一处理流式/非流式转发
    async fn handle(
        &self,
        processed: ProcessedRequest,
        access_point: &AccessPointEx,
        user_id: Uuid,
    ) -> Result<axum::response::Response, AppError> {
        let body_bytes =
            Bytes::from(serde_json::to_string(processed.outbound.body()).unwrap_or_default());
        let start = Instant::now();

        let log_ctx = LogContext::from_request(&processed, access_point, user_id);

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

        if processed.is_streaming {
            let log_service = self.log_service.clone();
            let log_buffer = Arc::new(std::sync::Mutex::new(String::new()));
            let log_buffer_clone = log_buffer.clone();
            let runtime = tokio::runtime::Handle::current();

            let byte_stream = upstream_resp.bytes_stream();

            let log_ctx_for_guard = log_ctx.clone();
            let resp_headers_for_response = resp_headers.clone();
            let resp_headers_for_guard = resp_headers.clone();

            let stream = async_stream::stream! {
                tokio::pin!(byte_stream);

                let mut guard = log_ctx_for_guard.into_interrupt_guard(
                    log_service.clone(),
                    status.as_u16(),
                    start,
                    log_buffer_clone,
                    resp_headers_for_guard,
                    runtime.clone(),
                );

                while let Some(chunk) = byte_stream.next().await {
                    match chunk {
                        Ok(bytes) => {
                            guard.buffer.lock().unwrap().push_str(&String::from_utf8_lossy(&bytes));
                            yield Ok(bytes);
                        }
                        Err(e) => {
                            yield Err(axum::Error::new(e));
                        }
                    }
                }

                guard.completed = true;
                let buf = std::mem::take(&mut *guard.buffer.lock().unwrap());
                if !buf.is_empty() {
                    let elapsed = start.elapsed();
                    spawn_log_task(log_ctx.into_log_task_context(
                        log_service,
                        status.as_u16(),
                        elapsed,
                        buf,
                        resp_headers,
                    ));
                }
            };

            let mut response_builder = axum::response::Response::builder()
                .status(status)
                .header("content-type", "text/event-stream")
                .header("cache-control", "no-cache")
                .header("connection", "keep-alive");

            for (key, value) in resp_headers_for_response.iter() {
                let key_str = key.as_str().to_lowercase();
                if key_str != "transfer-encoding"
                    && key_str != "content-type"
                    && key_str != "content-length"
                {
                    response_builder = response_builder.header(key, value.clone());
                }
            }

            response_builder
                .body(axum::body::Body::from_stream(stream))
                .map_err(|e| AppError::Internal(format!("构建流式响应失败: {}", e)))
        } else {
            let resp_body = upstream_resp
                .bytes()
                .await
                .map_err(|e| AppError::Upstream(format!("读取上游响应失败: {}", e)))?;

            let elapsed = start.elapsed();
            let resp_headers_for_response = resp_headers.clone();
            spawn_log_task(log_ctx.into_log_task_context(
                self.log_service.clone(),
                status.as_u16(),
                elapsed,
                String::from_utf8_lossy(&resp_body).to_string(),
                resp_headers,
            ));

            let mut response_builder = axum::response::Response::builder().status(status);
            for (key, value) in resp_headers_for_response.iter() {
                let key_str = key.as_str().to_lowercase();
                if key_str != "transfer-encoding" {
                    response_builder = response_builder.header(key, value.clone());
                }
            }

            response_builder
                .body(axum::body::Body::from(resp_body))
                .map_err(|e| AppError::Internal(format!("构建响应失败: {}", e)))
        }
    }
}
