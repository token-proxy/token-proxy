use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 审计日志领域实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub details: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

impl AuditLog {
    /// 创建新的审计日志
    pub fn new(
        user_id: Option<Uuid>,
        action: impl Into<String>,
        entity_type: impl Into<String>,
        entity_id: Option<Uuid>,
        details: Option<serde_json::Value>,
    ) -> Self {
        AuditLog {
            id: Uuid::new_v4(),
            user_id,
            action: action.into(),
            entity_type: entity_type.into(),
            entity_id,
            details,
            timestamp: Utc::now(),
        }
    }
}
