use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// API key 列表响应（脱敏，不返回完整 key）
#[derive(Debug, Clone, Serialize)]
pub struct UserApiKeyResponse {
    pub id: Uuid,
    /// 前缀（显示前 12 位 + `...`）
    pub key_prefix: String,
    pub description: String,
    pub status: String,
    /// 最后使用时间
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
