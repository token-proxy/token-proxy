use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── 请求 DTO ───

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProviderRequest {
    pub name: Option<String>,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
    pub models: Option<Vec<String>>,
    pub status: Option<String>,
}

// ─── 响应 DTO ───

#[derive(Debug, Clone, Serialize)]
pub struct ProviderResponse {
    pub id: Uuid,
    pub name: String,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
    pub models: Vec<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub account_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderSummary {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub account_count: i64,
}
