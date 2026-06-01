use axum::http::HeaderMap;
use serde_json::Value;

/// API 协议行为 — 协议特定的请求构造细节
///
/// 不同 LLM API 提供商（Anthropic、OpenAI）在请求头、模型字段位置、
/// 流式检测等方面存在差异。此 trait 将这些差异封装为协议实现，
/// 供 `ProxyPipeline` 在转发时根据接入点的 `api_type` 选择调用。
pub trait ApiProtocol: Send + Sync {
    /// 协议需要追加的额外上游 headers（如 anthropic-version）
    fn upstream_extra_headers(&self, api_key: &str) -> Vec<(&str, String)>;

    /// 请求体中模型字段的 JSON key 名（默认 "model"）
    fn model_key(&self) -> &str {
        "model"
    }

    /// 判断请求是否为流式（SSE）
    fn is_streaming(&self, body: &Value, headers: &HeaderMap) -> bool;
}
