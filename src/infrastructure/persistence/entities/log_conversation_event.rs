use sea_orm::entity::prelude::*;
use sea_orm::Set;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "log_conversation_events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub log_id: Uuid,
    pub session_id: String,
    pub timestamp: DateTimeWithTimeZone,
    pub request_index: i32,
    pub event_index: i32,
    pub parent_event_id: Option<Uuid>,
    pub parent_tool_use_id: Option<String>,
    pub source: String,
    pub role: String,
    pub event_type: String,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    pub tool_use_id: Option<String>,
    pub tool_name: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub content_preview: Option<String>,
    pub thinking_content: Option<String>,
    pub hidden_content: Option<Json>,
    pub display_payload: Option<Json>,
    pub confidence: i16,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

use crate::domain::entities::log_entry::LogConversationEvent;
use crate::shared::error::AppError;
use chrono::{FixedOffset, Utc};

impl TryFrom<Model> for LogConversationEvent {
    type Error = AppError;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        Ok(LogConversationEvent {
            id: model.id,
            log_id: model.log_id,
            session_id: model.session_id,
            timestamp: model.timestamp.with_timezone(&Utc),
            request_index: model.request_index,
            event_index: model.event_index,
            parent_event_id: model.parent_event_id,
            parent_tool_use_id: model.parent_tool_use_id,
            source: model.source,
            role: model.role,
            event_type: model.event_type,
            agent_id: model.agent_id,
            agent_type: model.agent_type,
            tool_use_id: model.tool_use_id,
            tool_name: model.tool_name,
            title: model.title,
            content: model.content,
            content_preview: model.content_preview,
            thinking_content: model.thinking_content,
            hidden_content: model.hidden_content,
            display_payload: model.display_payload,
            confidence: model.confidence,
            created_at: model.created_at.with_timezone(&Utc),
        })
    }
}

impl From<LogConversationEvent> for ActiveModel {
    fn from(event: LogConversationEvent) -> Self {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");

        ActiveModel {
            id: Set(event.id),
            log_id: Set(event.log_id),
            session_id: Set(event.session_id),
            timestamp: Set(event.timestamp.with_timezone(&offset)),
            request_index: Set(event.request_index),
            event_index: Set(event.event_index),
            parent_event_id: Set(event.parent_event_id),
            parent_tool_use_id: Set(event.parent_tool_use_id),
            source: Set(event.source),
            role: Set(event.role),
            event_type: Set(event.event_type),
            agent_id: Set(event.agent_id),
            agent_type: Set(event.agent_type),
            tool_use_id: Set(event.tool_use_id),
            tool_name: Set(event.tool_name),
            title: Set(event.title),
            content: Set(event.content),
            content_preview: Set(event.content_preview),
            thinking_content: Set(event.thinking_content),
            hidden_content: Set(event.hidden_content),
            display_payload: Set(event.display_payload),
            confidence: Set(event.confidence),
            created_at: Set(event.created_at.with_timezone(&offset)),
        }
    }
}
