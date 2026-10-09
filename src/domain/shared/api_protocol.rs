//! 上游 API 协议值对象 — domain/shared/
//!
//! `ApiProtocol` 描述「一次请求实际使用的上游 API 协议」，是代理管道转发行为的
//! 唯一决策依据。它与 `AccessPointType`（接入点类型，落库 + 前端展示）刻意分离：
//!
//! | 概念                | 回答的问题                       | 归属                       |
//! | ------------------- | -------------------------------- | -------------------------- |
//! | `AccessPointType`   | 这个接入点属于哪个协议家族？     | 实体字段（落库 / 前端 Select） |
//! | `ApiProtocol`       | 这一次请求走哪个上游协议？       | 值对象（请求级，运行期推导）   |
//!
//! 之所以拆开：OpenAI 有**两条线协议**共用一个服务商 base_url——
//! Chat Completions（`/v1/chat/completions`，`messages` 数组）与
//! Responses（`/v1/responses`，`input` 数组或字符串）。二者请求体形状、
//! 流式事件族、usage 字段族都不同，属于不同协议；但它们是同一个
//! `openai` 接入点的两种调用方式，不应迫使运维建两个接入点。
//!
//! 因此协议由**入站路径**推导（见 [`ApiProtocol::detect_openai`]），并由接入点类型
//! 做家族校验（`anthropic` 接入点只能走 Anthropic 协议）。
//!
//! 新增协议时：补本枚举 variant + 新建 `protocols/<name>.rs` +
//! `AccessPointType::allows` 守卫，编译器会指出所有待补 match 分支。

use axum::http::HeaderMap;
use serde_json::Value;

use crate::shared::error::AppError;

use super::inbound_request::InboundRequest;
use super::protocols::{anthropic, openai};

/// 上游 API 协议（请求级值对象）
///
/// 三个 variant 对应三套互不相同的线协议。`Display` 输出 snake_case 原始值，
/// 直接作为 `log_requests.api_protocol` 落库，无需额外映射表。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiProtocol {
    /// Anthropic Messages API（`/v1/messages`）
    Anthropic,
    /// OpenAI Chat Completions API（`/v1/chat/completions`）
    OpenAi,
    /// OpenAI Responses API（`/v1/responses`）
    OpenAiResponse,
}

impl ApiProtocol {
    /// 全部协议变体（用于遍历 / 文档化）
    pub fn all() -> &'static [ApiProtocol] {
        &[
            ApiProtocol::Anthropic,
            ApiProtocol::OpenAi,
            ApiProtocol::OpenAiResponse,
        ]
    }

    /// 稳定的 snake_case 原始值（落库 / 前端筛选使用）
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiProtocol::Anthropic => "anthropic",
            ApiProtocol::OpenAi => "openai",
            ApiProtocol::OpenAiResponse => "openai_response",
        }
    }

    /// 从入站路径推导本次请求使用的协议
    ///
    /// 判定步骤：
    /// 1. 路径含 `responses` → OpenAI Responses（比 Chat 更具体，优先判定）
    /// 2. 否则 → OpenAI Chat Completions（`/chat/completions` 及未知路径兜底）
    ///
    /// Anthropic 接入点不参与路径推导（其接入点类型已唯一确定协议），
    /// 由调用方直接使用 [`ApiProtocol::Anthropic`]。
    pub fn detect_openai(remainder: &str) -> ApiProtocol {
        if remainder.contains("responses") {
            ApiProtocol::OpenAiResponse
        } else {
            ApiProtocol::OpenAi
        }
    }

    /// 解析入站请求体，提取统一形态的模型名
    ///
    /// 各协议只负责「自己那种请求体怎么读出 model 字段」，返回的
    /// [`InboundRequest`] 已是协议无关的统一形态。
    pub fn parse_inbound(
        &self,
        headers: HeaderMap,
        body: String,
        remainder: &str,
    ) -> Result<InboundRequest, AppError> {
        match self {
            ApiProtocol::Anthropic => anthropic::parse_inbound(*self, headers, body),
            ApiProtocol::OpenAi => openai::parse_chat_inbound(*self, headers, body, remainder),
            ApiProtocol::OpenAiResponse => {
                openai::parse_responses_inbound(*self, headers, body, remainder)
            }
        }
    }

    /// 提取客户端会话标识（协议层兜底，`ClientType` 层优先）
    ///
    /// 返回 `None` 表示请求未携带会话标识。
    pub fn extract_session_id(&self, inbound: &InboundRequest) -> Option<String> {
        match self {
            ApiProtocol::Anthropic => anthropic::extract_session_id(&inbound.headers),
            // OpenAI 两条协议共用 `thread-id`（由 Codex 等客户端写入）
            ApiProtocol::OpenAi | ApiProtocol::OpenAiResponse => inbound
                .headers
                .get("thread-id")
                .and_then(|v| v.to_str().ok())
                .map(String::from),
        }
    }

    /// 向上游请求头注入 API key（不同协议 header 名/格式可能不同）
    pub fn inject_api_key(&self, headers: &mut HeaderMap, key: &str) {
        match self {
            ApiProtocol::Anthropic => anthropic::inject_api_key(headers, key),
            ApiProtocol::OpenAi | ApiProtocol::OpenAiResponse => {
                openai::inject_api_key(headers, key)
            }
        }
    }

    /// 替换请求体中的 model 字段（模型路由网格映射结果）
    ///
    /// 三条协议均为顶层 `model` 字符串字段，故共用实现；保留独立方法是为了
    /// 未来某协议改用嵌套位置时不牵连调用方。
    pub fn replace_model_in_body(&self, body: &Value, new_model: &str) -> Value {
        match self {
            ApiProtocol::Anthropic => anthropic::replace_model_in_body(body, new_model),
            ApiProtocol::OpenAi | ApiProtocol::OpenAiResponse => {
                openai::replace_model_in_body(body, new_model)
            }
        }
    }

    /// 判断上游响应是否为 SSE 流式响应（基于 `Content-Type`）
    pub fn is_sse_response(&self, resp_headers: &HeaderMap) -> bool {
        match self {
            ApiProtocol::Anthropic => anthropic::is_sse_response(resp_headers),
            ApiProtocol::OpenAi | ApiProtocol::OpenAiResponse => {
                openai::is_sse_response(resp_headers)
            }
        }
    }

    /// 按上游能力降级请求体中较新的消息 role（原地修改）
    ///
    /// 返回被改写的 role 数量（0 表示请求体未改动），供调用方记日志/审计。
    /// 仅对已知的「新命名 → 等价旧命名」映射生效；未知 role 一律保留原样，
    /// 保证降级是**白名单式**的、不会误伤。
    ///
    /// 当前规则（OpenAI 家族）：`developer` → `system`。
    /// Anthropic 协议无此落差，恒返回 0。
    ///
    /// 是否需要调用由 [`crate::domain::shared::UpstreamCompat`] 决定，
    /// 本方法只负责「怎么改」，不负责「要不要改」。
    pub fn normalize_legacy_roles(&self, body: &mut Value) -> usize {
        match self {
            ApiProtocol::Anthropic => 0,
            ApiProtocol::OpenAi | ApiProtocol::OpenAiResponse => {
                openai::normalize_legacy_roles(body)
            }
        }
    }
}

impl std::fmt::Display for ApiProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ApiProtocol {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "anthropic" => Ok(ApiProtocol::Anthropic),
            "openai" => Ok(ApiProtocol::OpenAi),
            "openai_response" | "openai_responses" => Ok(ApiProtocol::OpenAiResponse),
            _ => Err(AppError::Validation(format!("不支持的 API 协议: {}", s))),
        }
    }
}

impl serde::Serialize for ApiProtocol {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ApiProtocol {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn detect_responses_path_yields_responses_protocol() {
        assert_eq!(
            ApiProtocol::detect_openai("v1/responses"),
            ApiProtocol::OpenAiResponse
        );
    }

    #[test]
    fn detect_chat_path_yields_chat_protocol() {
        assert_eq!(
            ApiProtocol::detect_openai("v1/chat/completions"),
            ApiProtocol::OpenAi
        );
    }

    #[test]
    fn detect_unknown_path_defaults_to_chat() {
        assert_eq!(ApiProtocol::detect_openai("v1/models"), ApiProtocol::OpenAi);
    }

    #[test]
    fn as_str_is_stable_snake_case() {
        assert_eq!(ApiProtocol::Anthropic.as_str(), "anthropic");
        assert_eq!(ApiProtocol::OpenAi.as_str(), "openai");
        assert_eq!(ApiProtocol::OpenAiResponse.as_str(), "openai_response");
    }

    #[test]
    fn from_str_roundtrips_all_variants() {
        for p in ApiProtocol::all() {
            assert_eq!(ApiProtocol::from_str(p.as_str()).unwrap(), *p);
            assert_eq!(p.to_string(), p.as_str());
        }
    }

    #[test]
    fn from_str_accepts_hyphen_alias() {
        assert_eq!(
            ApiProtocol::from_str("openai-response").unwrap(),
            ApiProtocol::OpenAiResponse
        );
    }

    #[test]
    fn from_str_rejects_unknown() {
        assert!(ApiProtocol::from_str("gemini").is_err());
    }

    #[test]
    fn serde_roundtrip_uses_snake_case() {
        let json = serde_json::to_string(&ApiProtocol::OpenAiResponse).unwrap();
        assert_eq!(json, "\"openai_response\"");
        let back: ApiProtocol = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ApiProtocol::OpenAiResponse);
    }
}
