use std::sync::Arc;
use std::time::Instant;

use axum::http::HeaderMap;
use bytes::Bytes;
use futures::StreamExt;
use serde_json::{Map, Value};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::domain::entities::access_point::AccessPoint;
use crate::domain::entities::log_entry::LogEntry;
use crate::domain::repositories::access_point_repository::AccessPointRepository;
use crate::domain::services::{ApiProtocol, EncryptionService};
use crate::domain::value_objects::access_point_type::AccessPointType;
use crate::infrastructure::http_client::proxy_client::ProxyClient;
use crate::infrastructure::protocols::anthropic::AnthropicProtocol;
use crate::application::services::log_service::LogService;
use crate::shared::error::AppError;

/// 代理转发管道
///
/// 编排一次 LLM API 代理转发的完整流程：
/// 聚合加载 → 解密 → 模型匹配 → 请求变换 → 上游转发 → 日志记录
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
        let (access_point, provider, account) = self
            .access_point_repo
            .find_with_relations(short_code)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("接入点 '{}' 未找到", short_code)))?;

        if !access_point.status.is_enabled() {
            return Err(AppError::Forbidden(format!("接入点 '{}' 已被禁用", short_code)));
        }
        if !provider.status.is_enabled() {
            return Err(AppError::Forbidden(format!(
                "接入点 '{}' 关联的提供商已被禁用",
                short_code
            )));
        }
        if !account.status.is_enabled() {
            return Err(AppError::Forbidden(format!(
                "接入点 '{}' 关联的账号已被禁用",
                short_code
            )));
        }

        // 2. 解密上游 API key
        let upstream_key = self.decrypt_account_key(&account.api_key_encrypted).await?;

        // 3. 确定 base_url 并构造上游 URL
        let base_url = self.select_base_url(&access_point.api_type, &provider)?;
        let upstream_url = format!("{}/{}", base_url.trim_end_matches('/'), remainder);

        // 4. 解析请求体
        let mut body_value: Value = serde_json::from_str(&body)
            .map_err(|e| AppError::Validation(format!("请求体 JSON 解析失败: {}", e)))?;

        // 5. 模型匹配
        let protocol = protocol_for(&access_point.api_type);
        let model_key = protocol.model_key();
        let model_original = body_value
            .get(model_key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let requested_model = model_original.as_deref().unwrap_or("");
        let final_model = access_point.resolve_model(
            requested_model,
            provider.default_model.as_deref(),
        );

        // 6. 替换模型字段
        let model_changed = !requested_model.is_empty() && final_model != requested_model;
        if model_changed {
            if let Some(obj) = body_value.as_object_mut() {
                obj.insert(model_key.to_string(), Value::String(final_model.clone()));
            }
        }
        let modified_body = serde_json::to_string(&body_value)
            .map_err(|e| AppError::Internal(format!("序列化请求体失败: {}", e)))?;

        // 7. 流检测
        let is_streaming = protocol.is_streaming(&body_value, &headers);

        // 8. 构造上游 headers
        let upstream_headers = self.build_upstream_headers(
            &headers,
            &upstream_key,
            protocol.as_ref(),
        );

        if is_streaming {
            self.handle_streaming(
                &upstream_url,
                upstream_headers,
                modified_body,
                body,
                &headers,
                &access_point,
                model_original,
                final_model,
                user_id,
            )
            .await
        } else {
            self.handle_non_streaming(
                &upstream_url,
                upstream_headers,
                modified_body,
                body,
                &headers,
                &access_point,
                model_original,
                final_model,
                user_id,
            )
            .await
        }
    }

    /// 解密账户的 API key
    async fn decrypt_account_key(&self, encrypted: &[u8]) -> Result<String, AppError> {
        if encrypted.is_empty() {
            return Err(AppError::NotFound(
                "API Key 未正确存储，请删除并重新添加 Account".to_string(),
            ));
        }
        let decrypted = self
            .encryption_service
            .decrypt(encrypted)
            .await
            .map_err(|e| AppError::Encryption(e.to_string()))?;
        String::from_utf8(decrypted)
            .map_err(|_| AppError::Internal("API Key 解码失败: 非法的 UTF-8 格式".to_string()))
    }

    /// 根据 api_type 选择对应的 base_url
    fn select_base_url<'a>(
        &self,
        api_type: &AccessPointType,
        provider: &'a crate::domain::entities::provider::Provider,
    ) -> Result<&'a str, AppError> {
        match api_type {
            AccessPointType::Anthropic => provider
                .anthropic_base_url
                .as_deref()
                .ok_or_else(|| AppError::Internal("提供商未配置 Anthropic 基础 URL".to_string())),
        }
    }

    /// 构造上游请求 headers
    ///
    /// 从客户端 headers 转发非敏感、非 hop-by-hop 的业务头，
    /// 追加协议特定 headers，设置 `Authorization: Bearer <upstream_key>`。
    fn build_upstream_headers(
        &self,
        request_headers: &HeaderMap,
        upstream_key: &str,
        protocol: &dyn ApiProtocol,
    ) -> HeaderMap {
        let mut headers = HeaderMap::new();

        // 转发客户端业务 headers（排除敏感头和 hop-by-hop 头）
        for (key, value) in request_headers.iter() {
            let key_str = key.as_str().to_lowercase();
            if is_sensitive_header(&key_str) || is_hop_by_hop_header(&key_str) {
                continue;
            }
            headers.insert(key.clone(), value.clone());
        }

        // 追加协议特定 headers
        for (name, value) in protocol.upstream_extra_headers(upstream_key) {
            headers.insert(
                axum::http::HeaderName::from_bytes(name.as_bytes())
                    .expect("hardcoded header name"),
                axum::http::HeaderValue::from_str(&value).expect("hardcoded header value"),
            );
        }

        // 设置上游认证
        headers.insert(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_str(&format!("Bearer {}", upstream_key))
                .expect("硬编码的 header 值"),
        );

        headers
    }

    /// 非流式转发
    #[allow(clippy::too_many_arguments)]
    async fn handle_non_streaming(
        &self,
        upstream_url: &str,
        upstream_headers: HeaderMap,
        modified_body: String,
        original_body: String,
        request_headers: &HeaderMap,
        access_point: &AccessPoint,
        model_original: Option<String>,
        model_mapped: String,
        user_id: Uuid,
    ) -> Result<axum::response::Response, AppError> {
        let start = Instant::now();
        let body_bytes = Bytes::from(modified_body);

        let (status, resp_headers, resp_body) = self
            .proxy_client
            .forward(upstream_url, upstream_headers, body_bytes)
            .await?;

        let duration = start.elapsed();

        // 异步记录日志
        let log_service = self.log_service.clone();
        let resp_body_clone = resp_body.clone();
        let request_headers_json = headers_to_json(request_headers);
        let access_point_id = access_point.id;
        let provider_id = access_point.provider_id;
        let account_id = access_point.account_id;
        let session_id = extract_session_id(request_headers);

        tokio::spawn(async move {
            let log_entry = LogEntry::new_proxy_entry();
            let log_entry = LogEntry {
                id: Uuid::new_v4(),
                session_id,
                user_id: Some(user_id),
                access_point_id: Some(access_point_id),
                provider_id: Some(provider_id),
                account_id: Some(account_id),
                model_original,
                model_mapped: Some(model_mapped),
                status_code: Some(status.as_u16() as i16),
                duration_ms: Some(duration.as_millis() as i32),
                error_message: None,
                ..log_entry
            };

            log_service
                .record_proxy_log(
                    log_entry,
                    request_headers_json,
                    serde_json::from_str(&original_body).unwrap_or_default(),
                    String::from_utf8_lossy(&resp_body_clone).to_string(),
                )
                .await
                .ok();
        });

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
    #[allow(clippy::too_many_arguments)]
    async fn handle_streaming(
        &self,
        upstream_url: &str,
        upstream_headers: HeaderMap,
        modified_body: String,
        original_body: String,
        request_headers: &HeaderMap,
        access_point: &AccessPoint,
        model_original: Option<String>,
        model_mapped: String,
        user_id: Uuid,
    ) -> Result<axum::response::Response, AppError> {
        let body_bytes = Bytes::from(modified_body);
        let start = Instant::now();
        let request_headers_json = headers_to_json(request_headers);

        let upstream_resp = self
            .proxy_client
            .forward_streaming(upstream_url, upstream_headers, body_bytes)
            .await?;

        let status = upstream_resp.status();
        let resp_headers = upstream_resp.headers().clone();

        let log_service = self.log_service.clone();
        let log_buffer: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
        let log_buffer_clone = log_buffer.clone();

        // 在 stream 外部提取所需的字段（stream 块需要 owned 值）
        let ap_id = access_point.id;
        let provider_id = access_point.provider_id;
        let account_id = access_point.account_id;
        let sess = extract_session_id(request_headers);

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
                    let log_entry = LogEntry::new_proxy_entry();
                    let log_entry = LogEntry {
                        id: Uuid::new_v4(),
                        session_id: sess,
                        user_id: Some(user_id),
                        access_point_id: Some(ap_id),
                        provider_id: Some(provider_id),
                        account_id: Some(account_id),
                        model_original,
                        model_mapped: Some(model_mapped),
                        status_code: Some(status.as_u16() as i16),
                        duration_ms: Some(elapsed.as_millis() as i32),
                        error_message: None,
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
}

/// 根据接入点类型选择对应的协议实现
fn protocol_for(api_type: &AccessPointType) -> Box<dyn ApiProtocol> {
    match api_type {
        AccessPointType::Anthropic => Box::new(AnthropicProtocol),
    }
}

/// 从请求头提取 session_id
fn extract_session_id(headers: &HeaderMap) -> String {
    headers
        .get("x-claude-code-session-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
}

/// 将 HeaderMap 转换为 JSON Value（敏感头脱敏）
fn headers_to_json(headers: &HeaderMap) -> Value {
    let mut map = Map::new();
    for (key, value) in headers {
        let name = key.as_str();
        let header_value = if is_sensitive_header(&name.to_lowercase()) {
            Value::String("[REDACTED]".to_string())
        } else {
            Value::String(value.to_str().unwrap_or("[non-UTF8]").to_string())
        };
        map.insert(name.to_string(), header_value);
    }
    Value::Object(map)
}

fn is_sensitive_header(name: &str) -> bool {
    name.eq_ignore_ascii_case("authorization")
        || name.eq_ignore_ascii_case("x-api-key")
        || name.eq_ignore_ascii_case("proxy-authorization")
        || name.eq_ignore_ascii_case("cookie")
        || name.eq_ignore_ascii_case("set-cookie")
}

fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name,
        "host" | "transfer-encoding" | "content-length" | "connection"
    )
}
