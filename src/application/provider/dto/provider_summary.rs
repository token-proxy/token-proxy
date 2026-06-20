use serde::Serialize;
use uuid::Uuid;

/// 服务商摘要 DTO（用于列表场景，不含配置详情）
#[derive(Debug, Clone, Serialize)]
pub struct ProviderSummary {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub account_count: i64,
}
