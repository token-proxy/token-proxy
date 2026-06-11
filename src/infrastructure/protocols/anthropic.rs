use axum::http::HeaderMap;
use serde_json::Value;

use crate::domain::shared::ApiProtocol;
use crate::shared::error::AppError;

/// Anthropic Messages API 协议实现
///
/// 封装 Anthropic API 的请求结构知识：
/// - 模型字段位于 `body["model"]`
/// - 通过 `body["stream"] == true` 和 `accept: text/event-stream` 检测流式请求
pub struct AnthropicApiProtocol;

impl ApiProtocol for AnthropicApiProtocol {
    fn extract_model(&self, body: &Value) -> Result<String, AppError> {
        body.get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::Validation("请求体缺少 model 字段".to_string()))
    }

    fn replace_model(&self, body: &mut Value, model: &str) {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("model".to_string(), Value::String(model.to_string()));
        }
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
