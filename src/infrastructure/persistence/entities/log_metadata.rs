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
    pub request_index: i32,
    pub client_session_id: Option<String>,
    pub client_app: Option<String>,
    pub client_user_agent: Option<String>,
    pub conversation_source: String,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    pub parent_agent_tool_use_id: Option<String>,
    pub request_kind: Option<String>,
    pub primary_tool_name: Option<String>,
    pub message_preview: Option<String>,
    pub message_full: Option<String>,
    pub response_preview: Option<String>,
    pub has_thinking: bool,
    pub has_tool_use: bool,
    pub has_error: bool,
    pub raw_content_available: bool,
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
            request_index: model.request_index,
            client_session_id: model.client_session_id,
            client_app: model.client_app,
            client_user_agent: model.client_user_agent,
            conversation_source: model.conversation_source,
            agent_id: model.agent_id,
            agent_type: model.agent_type,
            parent_agent_tool_use_id: model.parent_agent_tool_use_id,
            request_kind: model.request_kind,
            primary_tool_name: model.primary_tool_name,
            message_preview: model.message_preview,
            message_full: model.message_full,
            response_preview: model.response_preview,
            has_thinking: model.has_thinking,
            has_tool_use: model.has_tool_use,
            has_error: model.has_error,
            raw_content_available: model.raw_content_available,
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
            request_index: Set(entry.request_index),
            client_session_id: Set(entry.client_session_id),
            client_app: Set(entry.client_app),
            client_user_agent: Set(entry.client_user_agent),
            conversation_source: Set(entry.conversation_source),
            agent_id: Set(entry.agent_id),
            agent_type: Set(entry.agent_type),
            parent_agent_tool_use_id: Set(entry.parent_agent_tool_use_id),
            request_kind: Set(entry.request_kind),
            primary_tool_name: Set(entry.primary_tool_name),
            message_preview: Set(entry.message_preview),
            message_full: Set(entry.message_full),
            response_preview: Set(entry.response_preview),
            has_thinking: Set(entry.has_thinking),
            has_tool_use: Set(entry.has_tool_use),
            has_error: Set(entry.has_error),
            raw_content_available: Set(entry.raw_content_available),
        }
    }
}
