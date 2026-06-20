//! 日志元数据实体 — domain/log/
//!
//! 定义 `LogMetadata`（SeaORM 实体映射 `log_metadata` 表），
//! 每月分区，记录每次代理请求的核心元数据（会话、模型、耗时、状态码等）。
//! 列表查询基于此表，内容详情按需加载。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

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
    pub client_app: Option<String>,
    pub client_user_agent: Option<String>,
    pub conversation_source: String,
    pub agent_id: Option<String>,
    pub has_error: bool,
    pub raw_content_available: bool,
    /// 客户端是否中途断开连接
    pub is_interrupted: bool,
    /// 客户端名称（从 user-agent 解析）
    pub client_name: Option<String>,
    /// 客户端版本号
    pub client_version: Option<String>,
    /// 客户端发布渠道
    pub client_channel: Option<String>,
    /// 客户端平台
    pub client_platform: Option<String>,
    /// API 类型（Anthropic / OpenAI 等）
    pub api_type: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::content::Entity",
        from = "Column::Id",
        to = "super::content::Column::LogId"
    )]
    LogContent,
}

impl ActiveModelBehavior for ActiveModel {}

// ─── 领域行为 ──────────────────────────────────────────────────────

impl Model {
    /// 获取 timestamp 为 DateTime<Utc>
    pub fn timestamp_utc(&self) -> DateTime<Utc> {
        self.timestamp.with_timezone(&Utc)
    }
}
