use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

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
