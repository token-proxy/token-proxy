use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// 日志完整详情响应（含客户端信息、token 用量等）
#[derive(Debug, Clone, Serialize)]
pub struct LogDetailFullResponse {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub access_point_id: Option<Uuid>,
    pub access_point_name: Option<String>,
    pub provider_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub model_original: String,
    pub model_mapped: String,
    pub status_code: i32,
    pub duration_ms: i32,
    pub error_message: Option<String>,
    pub conversation_source: String,
    pub agent_id: Option<String>,
    /// 客户端版本号（从 User-Agent 解析的版本号段）
    pub client_version: Option<String>,
    pub api_type: Option<String>,
    pub request_headers: serde_json::Value,
    pub response_headers: serde_json::Value,
    pub request_body: serde_json::Value,
    pub response_body: String,
    pub token_input_tokens: Option<i32>,
    pub token_output_tokens: Option<i32>,
    pub token_cache_creation_input_tokens: Option<i32>,
    pub token_cache_read_input_tokens: Option<i32>,
    pub token_thinking_tokens: Option<i32>,
    pub token_total_tokens: Option<i32>,
    pub token_raw_usage: Option<serde_json::Value>,
}
