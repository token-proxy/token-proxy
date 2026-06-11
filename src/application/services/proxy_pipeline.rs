use std::sync::Arc;
use std::time::Instant;

use axum::http::HeaderMap;
use bytes::Bytes;
use futures::StreamExt;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::domain::access_point::AccessPointEx;
use crate::domain::log::LogEntry;
use crate::domain::access_point::repository::AccessPointRepository;
use crate::domain::shared::EncryptionService;
use crate::infrastructure::http_client::proxy_client::ProxyClient;
use crate::infrastructure::http_client::request_transform::ProcessedRequest;
use crate::application::services::log_service::LogService;
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
        // 1. 加载聚合
        let access_point = self
            .access_point_repo
            .find_by_short_code(short_code)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("接入点 '{}' 未找到", short_code)))?;

        access_point.validate_usable()?;

        // 2. 解密上游 API key
        let upstream_key = access_point.decrypt_upstream_key(&*self.encryption_service).await?;

        // 3. 变换入站请求 → 上游请求
        let processed = ProcessedRequest::prepare(
            &access_point,
            &upstream_key,
            remainder,
            headers,
            &body,
        )?;

        // 4. 分流：流式 / 非流式转发
        if processed.is_streaming {
            self.handle_streaming(processed, &access_point, user_id).await
        } else {
            self.handle_non_streaming(processed, &access_point, user_id).await
        }
    }

    /// 非流式转发
    async fn handle_non_streaming(
        &self,
        processed: ProcessedRequest,
        access_point: &AccessPointEx,
        user_id: Uuid,
    ) -> Result<axum::response::Response, AppError> {
        let start = Instant::now();
        let body_bytes = Bytes::from(processed.modified_body);

        // 在移动 upstream_headers 之前提取日志所需字段
        let log_entry = build_log_entry(
            access_point.id,
            access_point.provider_id,
            access_point.account_id,
            user_id,
            processed.session_id.clone(),
            processed.original_model.clone(),
            processed.mapped_model.clone(),
        );
        let request_headers_json = processed.request_headers_json.clone();
        let original_body = processed.original_body.clone();

        let (status, resp_headers, resp_body) = self
            .proxy_client
            .forward(&processed.upstream_url, processed.upstream_headers, body_bytes)
            .await?;

        let duration = start.elapsed();

        // 异步记录日志
        self.spawn_log_task(
            log_entry,
            request_headers_json,
            original_body,
            status.as_u16() as i16,
            duration.as_millis() as i32,
            String::from_utf8_lossy(&resp_body).to_string(),
        );

        // 构建响应
        let mut response_builder = axum::response::Response::builder().status(status);
        for (key, value) in resp_headers.iter() {
            let key_str = key.as_str().to_lowercase();
            if key_str != "transfer-encoding" {
                response_builder = response_builder.header(key, value.clone());
            }
        }

        response_builder
            .body(axum::body::Body::from(resp_body))
            .map_err(|e| AppError::Internal(format!("构建响应失败: {}", e)))
    }

    /// SSE 流式转发
    async fn handle_streaming(
        &self,
        processed: ProcessedRequest,
        access_point: &AccessPointEx,
        user_id: Uuid,
    ) -> Result<axum::response::Response, AppError> {
        let body_bytes = Bytes::from(processed.modified_body);
        let start = Instant::now();

        let upstream_resp = self
            .proxy_client
            .forward_streaming(&processed.upstream_url, processed.upstream_headers, body_bytes)
            .await?;

        let status = upstream_resp.status();
        let resp_headers = upstream_resp.headers().clone();

        let log_service = self.log_service.clone();
        let log_buffer: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
        let log_buffer_clone = log_buffer.clone();

        // 提取 stream 所需的 owned 值
        let ap_id = access_point.id;
        let provider_id = access_point.provider_id;
        let account_id = access_point.account_id;
        let original_model = processed.original_model.clone();
        let mapped_model = processed.mapped_model.clone();
        let session_id = processed.session_id.clone();
        let request_headers_json = processed.request_headers_json.clone();
        let original_body = processed.original_body.clone();

        let byte_stream = upstream_resp.bytes_stream();

        let stream = async_stream::stream! {
            tokio::pin!(byte_stream);

            while let Some(chunk) = byte_stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes).to_string();
                        log_buffer_clone.lock().await.push_str(&text);
                        yield Ok::<Bytes, axum::Error>(bytes);
                    }
                    Err(e) => {
                        yield Err(axum::Error::new(e));
                    }
                }
            }

            let buf = log_buffer_clone.lock().await.clone();
            if !buf.is_empty() {
                let elapsed = start.elapsed();
                let log_svc = log_service.clone();

                tokio::spawn(async move {
                    let log_entry = build_log_entry(
                        ap_id,
                        provider_id,
                        account_id,
                        user_id,
                        session_id,
                        original_model,
                        mapped_model,
                    );
                    let log_entry = LogEntry {
                        status_code: Some(status.as_u16() as i16),
                        duration_ms: Some(elapsed.as_millis() as i32),
                        ..log_entry
                    };

                    log_svc
                        .record_proxy_log(
                            log_entry,
                            request_headers_json,
                            serde_json::from_str(&original_body).unwrap_or_default(),
                            buf,
                        )
                        .await
                        .ok();
                });
            }
        };

        // 构建流式响应
        let mut response_builder = axum::response::Response::builder()
            .status(status)
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .header("connection", "keep-alive");

        for (key, value) in resp_headers.iter() {
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
    }

    /// 异步写日志（非流式）
    fn spawn_log_task(
        &self,
        log_entry: LogEntry,
        request_headers_json: serde_json::Value,
        original_body: String,
        status_code: i16,
        duration_ms: i32,
        resp_body: String,
    ) {
        let log_service = self.log_service.clone();
        let log_entry = LogEntry {
            status_code: Some(status_code),
            duration_ms: Some(duration_ms),
            ..log_entry
        };
        let original_body_parsed = serde_json::from_str(&original_body).unwrap_or_default();

        tokio::spawn(async move {
            log_service
                .record_proxy_log(log_entry, request_headers_json, original_body_parsed, resp_body)
                .await
                .ok();
        });
    }
}

/// 构造代理日志条目（不含 status_code / duration_ms，由调用方补充）
fn build_log_entry(
    access_point_id: Uuid,
    provider_id: Uuid,
    account_id: Uuid,
    user_id: Uuid,
    session_id: String,
    original_model: String,
    mapped_model: String,
) -> LogEntry {
    LogEntry {
        id: Uuid::new_v4(),
        session_id,
        user_id: Some(user_id),
        access_point_id: Some(access_point_id),
        provider_id: Some(provider_id),
        account_id: Some(account_id),
        model_original: Some(original_model),
        model_mapped: Some(mapped_model),
        error_message: None,
        ..LogEntry::new_proxy_entry()
    }
}
