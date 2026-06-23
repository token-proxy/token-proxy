//! OpenAI 协议适配实现 — domain/shared/protocols/openai.rs
//!
//! 支持 Chat Completions API（`/v1/chat/completions`）和
//! Responses API（`/v1/responses`）两种端点。
//! 内部通过 `remainder` 参数区分请求路径。
//!
//! 协议约定：
//! - Chat Completions 请求体：`{ "model": "...", "messages": [...], ... }`
//! - Responses API 请求体：`{ "model": "...", "input": [...] | "...", ... }`
//! - 客户端会话标识：`thread-id` header（由 ClientType 层先尝试，此为协议层兜底）
//! - 上游认证：`Authorization: Bearer <key>` header

use axum::http::HeaderMap;
use serde_json::Value;

use crate::domain::shared::AccessPointType;
use crate::shared::error::AppError;

use super::super::inbound_request::InboundRequest;

/// 解析入站请求体，提取模型名和请求参数。
///
/// 根据 `remainder` 路径选择解析策略：
/// - `/chat/completions` → 读 `model` + 验证 `messages` 数组存在
/// - `/responses` → 读 `model` + 验证 `input` 字段存在（数组或字符串）
///
/// # 参数
/// - `remainder`: 请求路径的剩余部分（不含 `/ap/<short_code>/` 前缀）
pub(in crate::domain::shared) fn parse_inbound(
    api_type: AccessPointType,
    headers: HeaderMap,
    body: String,
    remainder: &str,
) -> Result<InboundRequest, AppError> {
    let json: Value = serde_json::from_str(&body)
        .map_err(|e| AppError::Validation(format!("无效的 JSON 请求体: {}", e)))?;

    // 按路径分发
    if remainder.contains("responses") {
        parse_responses_inbound(api_type, headers, json)
    } else {
        // 默认走 Chat Completions 路径（包含 /chat/completions 及未知路径）
        parse_chat_inbound(api_type, headers, json)
    }
}

/// 解析 Chat Completions 请求体。
fn parse_chat_inbound(
    api_type: AccessPointType,
    headers: HeaderMap,
    json: Value,
) -> Result<InboundRequest, AppError> {
    let model = json
        .get("model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Validation("请求体缺少 model 字段".into()))?
        .to_string();

    // 验证 messages 数组存在
    if !json.get("messages").is_some_and(|v| v.is_array()) {
        return Err(AppError::Validation(
            "Chat Completions 请求体缺少 messages 数组".into(),
        ));
    }

    Ok(InboundRequest {
        api_type,
        headers,
        body: json,
        model,
    })
}

/// 解析 Responses API 请求体。
fn parse_responses_inbound(
    api_type: AccessPointType,
    headers: HeaderMap,
    json: Value,
) -> Result<InboundRequest, AppError> {
    let model = json
        .get("model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Validation("请求体缺少 model 字段".into()))?
        .to_string();

    // 验证 input 字段存在（可能是数组或字符串）
    match json.get("input") {
        Some(v) if v.is_array() || v.is_string() => {}
        _ => {
            return Err(AppError::Validation(
                "Responses API 请求体缺少 input 字段".into(),
            ));
        }
    }

    Ok(InboundRequest {
        api_type,
        headers,
        body: json,
        model,
    })
}

/// 注入 API 密钥到上游请求头。
///
/// OpenAI 协议使用 `Authorization: Bearer <key>` 格式。
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
/// 依据 `Content-Type` 是否包含 `text/event-stream`，与协议无关。
pub(in crate::domain::shared) fn is_sse_response(resp_headers: &HeaderMap) -> bool {
    resp_headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/event-stream"))
        .unwrap_or(false)
}
