use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
}

impl RefreshToken {
    /// 创建新的 RefreshToken
    pub fn new(user_id: Uuid, token_hash: String, expires_at: DateTime<Utc>) -> Self {
        RefreshToken {
            id: Uuid::new_v4(),
            user_id,
            token_hash,
            expires_at,
            revoked: false,
            created_at: Utc::now(),
        }
    }

    /// 判断 token 是否已过期
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    /// 判断 token 是否有效（未撤销且未过期）
    pub fn is_valid(&self) -> bool {
        !self.revoked && !self.is_expired()
    }
}
