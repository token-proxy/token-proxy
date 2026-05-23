use crate::domain::value_objects::status::Status;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub key_hash: String,
    pub key_prefix: String,
    pub description: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub status: Status,
    pub created_at: DateTime<Utc>,
}

impl UserApiKey {
    /// 创建新的 UserApiKey
    pub fn new(
        user_id: Uuid,
        key_hash: String,
        key_prefix: String,
        description: String,
    ) -> Self {
        UserApiKey {
            id: Uuid::new_v4(),
            user_id,
            key_hash,
            key_prefix,
            description,
            last_used_at: None,
            status: Status::Enabled,
            created_at: Utc::now(),
        }
    }
}