//! Anthropic 协议实现 — domain/shared/protocols/anthropic.rs
//!
//! 实现 Anthropic Messages API 的请求解析、session 提取、API key 注入和 body 模型替换。
//! 全部为 `pub(in crate::domain::shared) fn`，由 `AccessPointType` 的协议方法通过 match 分发调用。
//!
//! 协议约定：
//! - 请求体格式：`{ "model": "...", "messages": [...], ... }`
//! - 客户端会话标识：`x-claude-code-session-id` header
//! - 上游认证：`Authorization: Bearer <key>` header

use axum::http::HeaderMap;
use serde_json::Value;

use crate::domain::shared::AccessPointType;
use crate::shared::error::AppError;

use super::super::inbound_request::InboundRequest;

/// 解析入站请求，提取 model 字段
///
/// body 必须是合法 JSON 对象，必须包含 `model` 字段。
pub(in crate::domain::shared) fn parse_inbound(
    api_type: AccessPointType,
    headers: HeaderMap,
    body: String,
    _remainder: &str,
) -> Result<InboundRequest, AppError> {
    let body_json: Value = serde_json::from_str(&body)
        .map_err(|e| AppError::Validation(format!("请求体 JSON 解析失败: {}", e)))?;
    let model = body_json
        .get("model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Validation("请求体缺少 model 字段".to_string()))?
        .to_string();
    Ok(InboundRequest {
        api_type,
        headers,
        body: body_json,
        model,
    })
}

/// 提取 Claude Code 客户端写入的会话标识
///
/// 返回 `None` 表示请求未携带会话标识（例如非 Claude Code 客户端的直接调用）。
pub(in crate::domain::shared) fn extract_session_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-claude-code-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// 向上游请求头注入 API key
///
/// Anthropic 上游使用 `Authorization: Bearer <key>` 格式（与 Claude Code 上游协议匹配）。
/// 若已存在 Authorization 头会被覆盖。
pub(in crate::domain::shared) fn inject_api_key(headers: &mut HeaderMap, key: &str) {
    headers.insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_str(&format!("Bearer {}", key)).expect("硬编码 header 格式"),
    );
}

/// 替换请求体中的 `model` 字段（用于模型路由网格的映射结果）
///
/// 若 body 不是 JSON 对象则原样返回，不报错（解析时已校验过 model 字段，到达此处必为对象）。
pub(in crate::domain::shared) fn replace_model_in_body(body: &Value, new_model: &str) -> Value {
    let mut out = body.clone();
    if let Some(obj) = out.as_object_mut() {
        obj.insert("model".to_string(), Value::String(new_model.to_string()));
    }
    out
}

/// 判断上游响应是否为流式（SSE）响应
///
/// 依据 `Content-Type` 是否包含 `text/event-stream`，不基于请求特征预设。
pub(in crate::domain::shared) fn is_sse_response(resp_headers: &HeaderMap) -> bool {
    resp_headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/event-stream"))
        .unwrap_or(false)
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderName, HeaderValue};

    fn headers_from(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        h
    }

    #[test]
    fn parse_inbound_extracts_model() {
        let body = r#"{"model":"claude-sonnet-4-20250514","messages":[]}"#.to_string();
        let result = parse_inbound(AccessPointType::Anthropic, HeaderMap::new(), body, "");
        let inbound = result.expect("解析应成功");
        assert_eq!(inbound.model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn parse_inbound_rejects_invalid_json() {
        let result = parse_inbound(
            AccessPointType::Anthropic,
            HeaderMap::new(),
            "{".to_string(),
            "",
        );
        assert!(result.is_err());
    }

    #[test]
    fn parse_inbound_rejects_missing_model() {
        let body = r#"{"messages":[]}"#.to_string();
        let result = parse_inbound(AccessPointType::Anthropic, HeaderMap::new(), body, "");
        assert!(result.is_err());
    }

    #[test]
    fn extract_session_id_present_returns_some() {
        let headers = headers_from(&[("x-claude-code-session-id", "abc-123")]);
        assert_eq!(extract_session_id(&headers), Some("abc-123".to_string()));
    }

    #[test]
    fn extract_session_id_absent_returns_none() {
        assert_eq!(extract_session_id(&HeaderMap::new()), None);
    }

    #[test]
    fn inject_api_key_writes_bearer_authorization() {
        let mut headers = HeaderMap::new();
        inject_api_key(&mut headers, "sk-test-key");
        let auth = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap();
        assert_eq!(auth, "Bearer sk-test-key");
    }

    #[test]
    fn replace_model_in_body_overwrites_field() {
        let body: Value = serde_json::from_str(r#"{"model":"old","messages":[]}"#).unwrap();
        let new_body = replace_model_in_body(&body, "new");
        assert_eq!(new_body["model"], Value::String("new".to_string()));
        assert_eq!(new_body["messages"], Value::Array(vec![]));
    }

    #[test]
    fn is_sse_response_matches_text_event_stream() {
        let headers = headers_from(&[("content-type", "text/event-stream; charset=utf-8")]);
        assert!(is_sse_response(&headers));
    }

    #[test]
    fn is_sse_response_false_for_json() {
        let headers = headers_from(&[("content-type", "application/json")]);
        assert!(!is_sse_response(&headers));
    }
}
