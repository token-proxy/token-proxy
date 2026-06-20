use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// 账号响应体
///
/// 返回账号的基本信息和状态（不包含加密的 API Key）。
#[derive(Debug, Clone, Serialize)]
pub struct AccountResponse {
    pub id: Uuid,
    /// 所属服务商 ID
    pub provider_id: Uuid,
    pub name: String,
    /// API Key 末位后缀（用于识别，不返回完整 Key）
    pub api_key_suffix: String,
    /// 禁用原因（manual / rate_limited / balance_exhausted / fault）
    pub disabled_reason: Option<String>,
    /// 自动恢复时间（None 表示需要手动恢复）
    pub available_at: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
