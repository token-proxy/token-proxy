use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// 日志详情响应（含请求/响应内容）
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
