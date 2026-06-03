use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// SeaORM 实体映射 log_token_usage 表
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
    /// 服务端工具用量（JSONB: web_search_requests, web_fetch_requests）
    pub server_tool_usage: Option<Json>,
    /// 缓存创建详情（JSONB: ephemeral_5m_input_tokens, ephemeral_1h_input_tokens）
    pub cache_creation: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// 获取 timestamp 为 DateTime<Utc>
    pub fn timestamp_utc(&self) -> DateTime<Utc> {
        self.timestamp.with_timezone(&Utc)
    }

    /// 获取 created_at 为 DateTime<Utc>
    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.with_timezone(&Utc)
    }
}
