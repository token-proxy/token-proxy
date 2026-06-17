use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct ProviderSummary {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub account_count: i64,
}
