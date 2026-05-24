use crate::domain::value_objects::status::Status;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub provider_id: Uuid,
    pub name: String,
    pub api_key_suffix: String,
    pub status: Status,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Account {
    /// 创建新的 Account，仅存储 API Key 末尾 6 位
    pub fn new(provider_id: Uuid, name: String, api_key_suffix: String) -> Self {
        let now = Utc::now();
        Account {
            id: Uuid::new_v4(),
            provider_id,
            name,
            api_key_suffix,
            status: Status::Enabled,
            created_at: now,
            updated_at: now,
        }
    }
}
