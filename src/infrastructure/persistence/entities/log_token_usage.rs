use sea_orm::entity::prelude::*;
use sea_orm::Set;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "log_token_usage")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub log_id: Uuid,
    pub session_id: String,
    pub timestamp: DateTimeWithTimeZone,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    pub provider_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub model_original: Option<String>,
    pub model_mapped: Option<String>,
    pub conversation_source: Option<String>,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_creation_input_tokens: i32,
    pub cache_read_input_tokens: i32,
    pub thinking_tokens: i32,
    pub total_tokens: i32,
    pub raw_usage: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

use crate::domain::entities::log_entry::LogTokenUsage;
use crate::shared::error::AppError;
use chrono::{FixedOffset, Utc};

impl TryFrom<Model> for LogTokenUsage {
    type Error = AppError;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        Ok(LogTokenUsage {
            id: model.id,
            log_id: model.log_id,
            session_id: model.session_id,
            timestamp: model.timestamp.with_timezone(&Utc),
            user_id: model.user_id,
            access_point_id: model.access_point_id,
            provider_id: model.provider_id,
            account_id: model.account_id,
            model_original: model.model_original,
            model_mapped: model.model_mapped,
            conversation_source: model.conversation_source,
            agent_id: model.agent_id,
            agent_type: model.agent_type,
            input_tokens: model.input_tokens,
            output_tokens: model.output_tokens,
            cache_creation_input_tokens: model.cache_creation_input_tokens,
            cache_read_input_tokens: model.cache_read_input_tokens,
            thinking_tokens: model.thinking_tokens,
            total_tokens: model.total_tokens,
            raw_usage: model.raw_usage,
            created_at: model.created_at.with_timezone(&Utc),
        })
    }
}

impl From<LogTokenUsage> for ActiveModel {
    fn from(usage: LogTokenUsage) -> Self {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");

        ActiveModel {
            id: Set(usage.id),
            log_id: Set(usage.log_id),
            session_id: Set(usage.session_id),
            timestamp: Set(usage.timestamp.with_timezone(&offset)),
            user_id: Set(usage.user_id),
            access_point_id: Set(usage.access_point_id),
            provider_id: Set(usage.provider_id),
            account_id: Set(usage.account_id),
            model_original: Set(usage.model_original),
            model_mapped: Set(usage.model_mapped),
            conversation_source: Set(usage.conversation_source),
            agent_id: Set(usage.agent_id),
            agent_type: Set(usage.agent_type),
            input_tokens: Set(usage.input_tokens),
            output_tokens: Set(usage.output_tokens),
            cache_creation_input_tokens: Set(usage.cache_creation_input_tokens),
            cache_read_input_tokens: Set(usage.cache_read_input_tokens),
            thinking_tokens: Set(usage.thinking_tokens),
            total_tokens: Set(usage.total_tokens),
            raw_usage: Set(usage.raw_usage),
            created_at: Set(usage.created_at.with_timezone(&offset)),
        }
    }
}
