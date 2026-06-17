use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use super::token_usage_response::TokenUsageResponse;

/// 会话原始内容项（前端基于此构建事件流）
#[derive(Debug, Clone, Serialize)]
pub struct SessionContentItemResponse {
    pub log_id: Uuid,
    pub request_index: i32,
    pub timestamp: DateTime<Utc>,
    pub conversation_source: String,
    pub agent_id: Option<String>,
    pub request_headers: serde_json::Value,
    pub request_body: serde_json::Value,
    pub response_body: String,
    pub token_usage: Option<TokenUsageResponse>,
}
