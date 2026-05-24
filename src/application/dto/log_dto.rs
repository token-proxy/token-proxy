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
