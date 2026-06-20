use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// 创建 API key 响应（完整 key 仅在创建时返回一次）
#[derive(Debug, Clone, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    /// 完整 API key（仅创建时返回，后续不再暴露）
    pub full_key: String,
    /// 前缀（显示前 12 位 + `...`）
    pub key_prefix: String,
    pub description: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}
