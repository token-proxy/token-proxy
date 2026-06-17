use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

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
    pub client_name: Option<String>,
    pub client_version: Option<String>,
    pub client_channel: Option<String>,
    pub client_platform: Option<String>,
    pub api_type: String,
    pub token_input_tokens: Option<i32>,
    pub token_output_tokens: Option<i32>,
    pub token_total_tokens: Option<i32>,
}
