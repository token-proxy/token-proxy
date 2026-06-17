use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// API key 列表响应（脱敏，不返回完整 key）
#[derive(Debug, Clone, Serialize)]
pub struct UserApiKeyResponse {
    pub id: Uuid,
    pub key_prefix: String,
    pub description: String,
    pub status: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
