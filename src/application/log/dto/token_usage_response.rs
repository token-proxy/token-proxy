use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct TokenUsageResponse {
    pub id: Uuid,
    pub log_id: Uuid,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_creation_input_tokens: i32,
    pub cache_read_input_tokens: i32,
    pub thinking_tokens: i32,
    pub total_tokens: i32,
    pub raw_usage: Option<serde_json::Value>,
    pub server_tool_usage: Option<serde_json::Value>,
    pub cache_creation: Option<serde_json::Value>,
}
