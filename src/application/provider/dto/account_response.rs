use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct AccountResponse {
    pub id: Uuid,
    pub provider_id: Uuid,
    pub name: String,
    pub api_key_suffix: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
