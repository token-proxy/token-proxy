use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── 请求 DTO ───

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAccountRequest {
    pub name: Option<String>,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAccountRequest {
    pub name: Option<String>,
    pub api_key: Option<String>,
}

// ─── 响应 DTO ───

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
