//! 审计日志响应 DTO。

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// 审计日志列表响应项。
#[derive(Debug, Clone, Serialize)]
pub struct AuditLogResponse {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub operator_id: Option<Uuid>,
    /// 操作者类型（user / system）
    pub operator_type: String,
    /// 操作者用户名（来自 users 表 LEFT JOIN），系统操作时为 None
    pub operator_name: Option<String>,
    /// 操作类型（snake_case 原始值）
    pub action: String,
    /// 实体类型（snake_case 原始值）
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    /// 操作详情 JSON
    pub details: Option<serde_json::Value>,
}
