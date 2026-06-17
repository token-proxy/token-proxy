use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// 创建 API key 响应（完整 key 仅在创建时返回一次）
#[derive(Debug, Clone, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub full_key: String,
    pub key_prefix: String,
    pub description: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}
