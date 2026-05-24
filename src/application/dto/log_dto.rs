use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Request / Query DTOs ───

#[derive(Debug, Clone, Deserialize)]
pub struct LogFilterParams {
    pub session_id: Option<String>,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub status_code: Option<i16>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// ─── Response DTOs ───

#[derive(Debug, Clone, Serialize)]
pub struct LogSummaryResponse {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    pub model_original: Option<String>,
    pub model_mapped: Option<String>,
    pub status_code: Option<i16>,
    pub duration_ms: Option<i32>,
    pub conversation_source: String,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    pub request_kind: Option<String>,
    pub primary_tool_name: Option<String>,
    pub message_preview: Option<String>,
    pub message_full: Option<String>,
    pub response_preview: Option<String>,
    pub has_thinking: bool,
    pub has_tool_use: bool,
    pub raw_content_available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogDetailResponse {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    pub provider_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub model_original: Option<String>,
    pub model_mapped: Option<String>,
    pub status_code: Option<i16>,
    pub duration_ms: Option<i32>,
    pub error_message: Option<String>,
    pub request_headers: Option<serde_json::Value>,
    pub request_body: Option<serde_json::Value>,
    pub response_body: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionSummaryResponse {
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    pub start_time: DateTime<Utc>,
    pub request_count: u64,
    pub first_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConversationEventResponse {
    pub id: Uuid,
    pub log_id: Uuid,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub request_index: i32,
    pub event_index: i32,
    pub parent_event_id: Option<Uuid>,
    pub parent_tool_use_id: Option<String>,
    pub source: String,
    pub role: String,
    pub event_type: String,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    pub tool_use_id: Option<String>,
    pub tool_name: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub content_preview: Option<String>,
    pub thinking_content: Option<String>,
    pub hidden_content: Option<serde_json::Value>,
    pub display_payload: Option<serde_json::Value>,
    pub confidence: i16,
}

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
}
