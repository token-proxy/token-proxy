use axum::http::HeaderMap;
use serde_json::Value;

use crate::domain::shared::ApiProtocol;

/// Anthropic Messages API 协议实现
///
/// 处理 Anthropic 特定的请求构造细节：
/// - 追加 `anthropic-version: 2023-06-01` 头
/// - 通过 `body["stream"] == true` 和 `accept: text/event-stream` 检测流式请求
pub struct AnthropicProtocol;

impl ApiProtocol for AnthropicProtocol {
    fn upstream_extra_headers(&self, _api_key: &str) -> Vec<(&str, String)> {
        vec![("anthropic-version", "2023-06-01".to_string())]
    }

    fn is_streaming(&self, body: &Value, headers: &HeaderMap) -> bool {
        body.get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || headers
                .get("accept")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.contains("text/event-stream"))
                .unwrap_or(false)
    }
}
