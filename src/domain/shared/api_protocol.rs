use axum::http::HeaderMap;
use serde_json::Value;

pub trait ApiProtocol: Send + Sync {
    fn upstream_extra_headers(&self, api_key: &str) -> Vec<(&str, String)>;
    fn model_key(&self) -> &str { "model" }
    fn is_streaming(&self, body: &Value, headers: &HeaderMap) -> bool;
}
