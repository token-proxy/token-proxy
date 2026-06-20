use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// 用户响应体（不包含密码哈希）
#[derive(Debug, Clone, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    /// 显示名称
    pub display_name: String,
    /// 状态（enabled / disabled）
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
