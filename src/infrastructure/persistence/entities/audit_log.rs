use sea_orm::entity::prelude::*;

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

// ─── 领域模型转换 ────────────────────────────────────────────────

use crate::domain::entities::audit_log::AuditLog;
use crate::shared::error::AppError;
use chrono::Utc;

impl TryFrom<Model> for AuditLog {
    type Error = AppError;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        Ok(AuditLog {
            id: model.id,
            user_id: model.user_id,
            action: model.action,
            entity_type: model.entity_type,
            entity_id: model.entity_id,
            details: model.details,
            timestamp: model.timestamp.with_timezone(&Utc),
        })
    }
}
