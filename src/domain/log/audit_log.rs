use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// SeaORM 实体映射 audit_logs 表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "audit_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub details: Option<Json>,
    pub timestamp: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// 领域实体 AuditLog
pub type AuditLog = Model;

// ─── 领域行为 ──────────────────────────────────────────────────────

impl Model {
    /// 创建新的审计日志
    pub fn new(
        user_id: Option<Uuid>,
        action: impl Into<String>,
        entity_type: impl Into<String>,
        entity_id: Option<Uuid>,
        details: Option<serde_json::Value>,
    ) -> Self {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        AuditLog {
            id: Uuid::new_v4(),
            user_id,
            action: action.into(),
            entity_type: entity_type.into(),
            entity_id,
            details,
            timestamp: Utc::now().with_timezone(&offset),
        }
    }

    /// 获取 timestamp 为 DateTime<Utc>
    pub fn timestamp_utc(&self) -> DateTime<Utc> {
        self.timestamp.with_timezone(&Utc)
    }
}
