use crate::domain::value_objects::status::Status;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub status: Status,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// 创建新的 User
    pub fn new(username: String, display_name: String, password_hash: String) -> Self {
        let now = Utc::now();
        User {
            id: Uuid::new_v4(),
            username,
            display_name,
            password_hash,
            status: Status::Enabled,
            created_at: now,
            updated_at: now,
        }
    }
}
