use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

// ─── Request DTOs ───

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAccountRequest {
    pub name: Option<String>,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAccountRequest {
    pub name: Option<String>,
}

// ─── Response DTOs ───

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