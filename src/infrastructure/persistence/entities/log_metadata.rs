use sea_orm::entity::prelude::*;
use sea_orm::Set;

/// SeaORM 实体映射 log_metadata 表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "log_metadata")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub timestamp: DateTimeWithTimeZone,
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    pub provider_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub model_original: Option<String>,
    pub model_mapped: Option<String>,
    pub status_code: Option<i16>,
    pub duration_ms: Option<i32>,
    pub error_message: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::log_content::Entity",
        from = "Column::Id",
        to = "super::log_content::Column::LogId"
    )]
    LogContent,
}

impl ActiveModelBehavior for ActiveModel {}

// ─── 领域模型转换 ────────────────────────────────────────────────

use crate::domain::entities::log_entry::LogEntry;
use crate::shared::error::AppError;
use chrono::{FixedOffset, Utc};

impl TryFrom<Model> for LogEntry {
    type Error = AppError;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        Ok(LogEntry {
            id: model.id,
            timestamp: model.timestamp.with_timezone(&Utc),
            session_id: model.session_id,
            user_id: model.user_id,
            access_point_id: model.access_point_id,
            provider_id: model.provider_id,
            account_id: model.account_id,
            model_original: model.model_original,
            model_mapped: model.model_mapped,
            status_code: model.status_code,
            duration_ms: model.duration_ms,
            error_message: model.error_message,
        })
    }
}

impl From<LogEntry> for ActiveModel {
    fn from(entry: LogEntry) -> Self {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");

        ActiveModel {
            id: Set(entry.id),
            timestamp: Set(entry.timestamp.with_timezone(&offset)),
            session_id: Set(entry.session_id),
            user_id: Set(entry.user_id),
            access_point_id: Set(entry.access_point_id),
            provider_id: Set(entry.provider_id),
            account_id: Set(entry.account_id),
            model_original: Set(entry.model_original),
            model_mapped: Set(entry.model_mapped),
            status_code: Set(entry.status_code),
            duration_ms: Set(entry.duration_ms),
            error_message: Set(entry.error_message),
        }
    }
}