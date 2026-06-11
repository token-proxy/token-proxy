use axum::http::HeaderMap;
use serde_json::{Map, Value};

use crate::domain::access_point::AccessPointEx;
use crate::domain::shared::{AccessPointType, ApiProtocol};
use crate::infrastructure::protocols::anthropic::AnthropicApiProtocol;
use crate::shared::error::AppError;

/// 处理后的上游请求
///
/// 将原始入站请求变换为可直接发送给上游 API 的请求，
/// 内聚 URL 构造、body 解析、模型映射替换、流检测、header 构造等变换逻辑。
pub struct ProcessedRequest {
    pub upstream_url: String,
    pub upstream_headers: HeaderMap,
    pub modified_body: String,
    pub original_body: String,
    pub is_streaming: bool,
    pub original_model: String,
    pub mapped_model: String,
    pub session_id: String,
    pub request_headers_json: Value,
}

impl ProcessedRequest {
    /// 从原始入站请求变换为上游请求
    ///
    /// 执行步骤：构造 URL → 解析 body → 模型匹配/替换 → 流检测 → 构造 upstream headers
    pub fn prepare(
        access_point: &AccessPointEx,
        upstream_key: &str,
        remainder: &str,
        headers: HeaderMap,
        body: &str,
    ) -> Result<Self, AppError> {
        let base_url = access_point.base_url()?;
        let upstream_url = format!("{}/{}", base_url.trim_end_matches('/'), remainder);

        let mut body_value: Value = serde_json::from_str(body)
            .map_err(|e| AppError::Validation(format!("请求体 JSON 解析失败: {}", e)))?;

        let protocol = protocol_for(&access_point.api_type);

        let original_model = protocol.extract_model(&body_value)?;
        let mapped_model = access_point.resolve_model(&original_model);

        if mapped_model != original_model {
            protocol.replace_model(&mut body_value, &mapped_model);
        }

        let is_streaming = protocol.is_streaming(&body_value, &headers);
        let modified_body = serde_json::to_string(&body_value)
            .map_err(|e| AppError::Internal(format!("序列化请求体失败: {}", e)))?;

        let upstream_headers = build_upstream_headers(&headers, upstream_key);

        let session_id = extract_session_id(&headers);
        let request_headers_json = headers_to_json(&headers);

        Ok(ProcessedRequest {
            upstream_url,
            upstream_headers,
            modified_body,
            original_body: body.to_string(),
            is_streaming,
            original_model,
            mapped_model,
            session_id,
            request_headers_json,
        })
    }
}

// ─── 内部辅助函数 ──────────────────────────────────────────────────────

fn protocol_for(api_type: &AccessPointType) -> Box<dyn ApiProtocol> {
    match api_type {
        AccessPointType::Anthropic => Box::new(AnthropicApiProtocol),
    }
}

fn build_upstream_headers(request_headers: &HeaderMap, upstream_key: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();

    for (key, value) in request_headers.iter() {
        let key_str = key.as_str().to_lowercase();
        if is_sensitive_header(&key_str) || is_hop_by_hop_header(&key_str) {
            continue;
        }
        headers.insert(key.clone(), value.clone());
    }

    headers.insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_str(&format!("Bearer {}", upstream_key))
            .expect("硬编码的 header 值"),
    );

    headers
}

fn extract_session_id(headers: &HeaderMap) -> String {
    headers
        .get("x-claude-code-session-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
}

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
