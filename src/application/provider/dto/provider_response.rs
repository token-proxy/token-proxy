use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

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
