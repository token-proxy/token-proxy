//! OpenAI 协议适配实现 — domain/shared/protocols/openai.rs
//!
//! 承载 OpenAI 的**两条线协议**，二者共用服务商 base_url 与
//! `Authorization: Bearer <key>` 认证，但请求体形状与端点不同：
//!
//! | 协议            | 端点                   | 请求体特征                       |
//! | --------------- | ---------------------- | -------------------------------- |
//! | Chat Completions | `/v1/chat/completions` | `messages` 数组（`role`/`content`） |
//! | Responses        | `/v1/responses`        | `input` 数组或字符串              |
//!
//! 两条协议的**校验入口分离**（`parse_chat_inbound` / `parse_responses_inbound`），
//! 由 [`crate::domain::shared::ApiProtocol`] 按入站路径分发，不再依赖
//! `remainder.contains("responses")` 的隐式猜测。
//!
//! 客户端会话标识：`thread-id` header（由 `ClientType` 层先尝试，此为协议层兜底）。

use axum::http::HeaderMap;
use serde_json::Value;

use crate::domain::shared::ApiProtocol;
use crate::shared::error::AppError;

use super::super::inbound_request::InboundRequest;

/// 解析 Chat Completions 入站请求（`/v1/chat/completions`）
///
/// 校验：body 为合法 JSON 对象 + 含 `model` 字符串 + 含 `messages` 数组。
pub(in crate::domain::shared) fn parse_chat_inbound(
    protocol: ApiProtocol,
    headers: HeaderMap,
    body: String,
    _remainder: &str,
) -> Result<InboundRequest, AppError> {
    let json = parse_body_json(&body)?;
    let model = extract_model(&json)?;

    if !json.get("messages").is_some_and(|v| v.is_array()) {
        return Err(AppError::Validation(
            "Chat Completions 请求体缺少 messages 数组".into(),
        ));
    }

    Ok(InboundRequest {
        protocol,
        headers,
        body: json,
        model,
    })
}

/// 解析 Responses 入站请求（`/v1/responses`）
///
/// 校验：body 为合法 JSON 对象 + 含 `model` 字符串 + 含 `input`（数组或字符串）。
///
/// 额外用成熟的 `async-openai` 类型做一次**结构探测**（见
/// [`probe_responses_request`]），失败仅记 debug 日志、不阻断转发——
/// 代理必须对上游新增字段保持透明，不能因 SDK 落后于 OpenAI 而拒绝合法请求。
pub(in crate::domain::shared) fn parse_responses_inbound(
    protocol: ApiProtocol,
    headers: HeaderMap,
    body: String,
    _remainder: &str,
) -> Result<InboundRequest, AppError> {
    let json = parse_body_json(&body)?;
    let model = extract_model(&json)?;

    match json.get("input") {
        Some(v) if v.is_array() || v.is_string() => {}
        _ => {
            return Err(AppError::Validation(
                "Responses API 请求体缺少 input 字段".into(),
            ));
        }
    }

    probe_responses_request(&json);

    Ok(InboundRequest {
        protocol,
        headers,
        body: json,
        model,
    })
}

/// 解析请求体为 JSON 对象
fn parse_body_json(body: &str) -> Result<Value, AppError> {
    serde_json::from_str(body)
        .map_err(|e| AppError::Validation(format!("无效的 JSON 请求体: {}", e)))
}

/// 从请求体中提取 `model` 字段
fn extract_model(json: &Value) -> Result<String, AppError> {
    json.get("model")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Validation("请求体缺少 model 字段".into()))
}

/// 用 `async-openai` 的 Responses 请求类型做结构探测（仅诊断用）
///
/// 目的：借成熟 SDK 的类型系统发现「我们手工校验覆盖不到」的结构性异常，
/// 例如 `input` 项形状非法。**探测失败不阻断**——代理的契约是透明转发，
/// SDK 版本滞后于 OpenAI 是常态，不能因此拒绝客户端请求。
fn probe_responses_request(json: &Value) {
    match serde_json::from_value::<async_openai::types::responses::CreateResponse>(json.clone()) {
        Ok(_) => tracing::trace!("Responses 请求体通过 SDK 结构校验"),
        Err(e) => tracing::debug!(
            error = %e,
            "Responses 请求体未通过 SDK 结构校验，将按原始 JSON 透明转发",
        ),
    }
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
/// 依据 `Content-Type` 是否包含 `text/event-stream`，两条 OpenAI 协议一致。
pub(in crate::domain::shared) fn is_sse_response(resp_headers: &HeaderMap) -> bool {
    resp_headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/event-stream"))
        .unwrap_or(false)
}

/// 按上游能力把较新的消息 role 降级为等价旧 role（原地修改）
///
/// 返回被改写的 role 数量。仅处理白名单内的已知映射，未知取值原样保留。
///
/// 覆盖两条协议的两种承载位置：
/// - Chat Completions：顶层 `messages` 数组的 `role` 字段
/// - Responses：`input` 数组的 `role` 字段（`input` 为字符串时无 role，天然无需处理）
///
/// 现有唯一规则：`developer` → `system`（OpenAI 规范中二者语义等价）。
pub(in crate::domain::shared) fn normalize_legacy_roles(body: &mut Value) -> usize {
    let mut changed = 0;

    // 1. Chat Completions：messages[].role
    if let Some(messages) = body.get_mut("messages").and_then(|v| v.as_array_mut()) {
        changed += normalize_role_field(messages);
    }

    // 2. Responses：input[].role
    if let Some(input) = body.get_mut("input").and_then(|v| v.as_array_mut()) {
        changed += normalize_role_field(input);
    }

    changed
}

/// 把数组元素中的 `role: "developer"` 改写为 `"system"`，返回改动条数
fn normalize_role_field(items: &mut [Value]) -> usize {
    let mut changed = 0;
    for item in items.iter_mut() {
        let Some(obj) = item.as_object_mut() else {
            continue;
        };
        let is_developer = obj
            .get("role")
            .and_then(Value::as_str)
            .is_some_and(|r| r.eq_ignore_ascii_case("developer"));
        if is_developer {
            obj.insert("role".to_string(), Value::String("system".to_string()));
            changed += 1;
        }
    }
    changed
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn chat_body() -> String {
        r#"{"model":"gpt-4o","messages":[{"role":"user","content":"hi"}]}"#.to_string()
    }

    fn responses_body() -> String {
        r#"{"model":"gpt-5","input":[{"role":"user","content":"hi"}]}"#.to_string()
    }

    #[test]
    fn parse_chat_accepts_messages_array() {
        let inbound = parse_chat_inbound(
            ApiProtocol::OpenAi,
            HeaderMap::new(),
            chat_body(),
            "v1/chat/completions",
        )
        .expect("应解析成功");
        assert_eq!(inbound.model, "gpt-4o");
        assert_eq!(inbound.protocol, ApiProtocol::OpenAi);
    }

    #[test]
    fn parse_chat_rejects_missing_messages() {
        let body = r#"{"model":"gpt-4o"}"#.to_string();
        assert!(parse_chat_inbound(
            ApiProtocol::OpenAi,
            HeaderMap::new(),
            body,
            "v1/chat/completions"
        )
        .is_err());
    }

    #[test]
    fn parse_chat_rejects_missing_model() {
        let body = r#"{"messages":[]}"#.to_string();
        assert!(parse_chat_inbound(ApiProtocol::OpenAi, HeaderMap::new(), body, "x").is_err());
    }

    #[test]
    fn parse_responses_accepts_input_array() {
        let inbound = parse_responses_inbound(
            ApiProtocol::OpenAiResponse,
            HeaderMap::new(),
            responses_body(),
            "v1/responses",
        )
        .expect("应解析成功");
        assert_eq!(inbound.model, "gpt-5");
        assert_eq!(inbound.protocol, ApiProtocol::OpenAiResponse);
    }

    #[test]
    fn parse_responses_accepts_input_string() {
        let body = r#"{"model":"gpt-5","input":"hello"}"#.to_string();
        assert!(parse_responses_inbound(
            ApiProtocol::OpenAiResponse,
            HeaderMap::new(),
            body,
            "v1/responses"
        )
        .is_ok());
    }

    #[test]
    fn parse_responses_rejects_missing_input() {
        let body = r#"{"model":"gpt-5","messages":[]}"#.to_string();
        assert!(parse_responses_inbound(
            ApiProtocol::OpenAiResponse,
            HeaderMap::new(),
            body,
            "v1/responses"
        )
        .is_err());
    }

    #[test]
    fn parse_rejects_invalid_json() {
        assert!(
            parse_chat_inbound(ApiProtocol::OpenAi, HeaderMap::new(), "{".into(), "x").is_err()
        );
    }

    #[test]
    fn responses_with_unknown_field_is_still_forwarded() {
        // SDK 类型滞后于上游时不得拒绝合法请求（透明转发契约）
        let body = r#"{"model":"gpt-5","input":"hi","brand_new_field":{"x":1}}"#.to_string();
        let inbound = parse_responses_inbound(
            ApiProtocol::OpenAiResponse,
            HeaderMap::new(),
            body,
            "v1/responses",
        )
        .expect("未知字段不应阻断");
        assert_eq!(inbound.model, "gpt-5");
    }

    #[test]
    fn inject_api_key_writes_bearer_authorization() {
        let mut headers = HeaderMap::new();
        inject_api_key(&mut headers, "sk-test");
        assert_eq!(
            headers
                .get(axum::http::header::AUTHORIZATION)
                .unwrap()
                .to_str()
                .unwrap(),
            "Bearer sk-test"
        );
    }

    #[test]
    fn replace_model_only_touches_model_field() {
        let body: Value = serde_json::from_str(&responses_body()).unwrap();
        let out = replace_model_in_body(&body, "gpt-5-mini");
        assert_eq!(out["model"], Value::String("gpt-5-mini".into()));
        assert!(out["input"].is_array());
    }

    #[test]
    fn is_sse_response_matches_event_stream() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "text/event-stream".parse().unwrap());
        assert!(is_sse_response(&headers));
    }

    #[test]
    fn is_sse_response_false_for_json() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        assert!(!is_sse_response(&headers));
    }

    // ── 旧 role 降级 ──

    #[test]
    fn normalize_legacy_roles_rewrites_chat_developer() {
        let mut body: Value = serde_json::from_str(
            r#"{"model":"gpt-4o","messages":[{"role":"developer","content":"x"},{"role":"user","content":"y"}]}"#,
        )
        .unwrap();
        assert_eq!(normalize_legacy_roles(&mut body), 1);
        assert_eq!(body["messages"][0]["role"], Value::String("system".into()));
        assert_eq!(body["messages"][1]["role"], Value::String("user".into()));
    }

    #[test]
    fn normalize_legacy_roles_rewrites_responses_input() {
        let mut body: Value = serde_json::from_str(
            r#"{"model":"gpt-5","input":[{"role":"developer","content":"x"}]}"#,
        )
        .unwrap();
        assert_eq!(normalize_legacy_roles(&mut body), 1);
        assert_eq!(body["input"][0]["role"], Value::String("system".into()));
    }

    #[test]
    fn normalize_legacy_roles_keeps_unknown_roles() {
        // 白名单式降级：未知 role 一律保留，不误伤
        let mut body: Value = serde_json::from_str(
            r#"{"model":"gpt-4o","messages":[{"role":"latest_reminder","content":"x"}]}"#,
        )
        .unwrap();
        assert_eq!(normalize_legacy_roles(&mut body), 0);
        assert_eq!(
            body["messages"][0]["role"],
            Value::String("latest_reminder".into())
        );
    }

    #[test]
    fn normalize_legacy_roles_is_noop_when_absent() {
        let mut body: Value = serde_json::from_str(r#"{"model":"gpt-5","input":"hello"}"#).unwrap();
        assert_eq!(normalize_legacy_roles(&mut body), 0);
    }
}
