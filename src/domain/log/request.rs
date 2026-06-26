//! 代理请求实体 — domain/log/
//!
//! 定义 `LogRequest`（SeaORM 实体映射 `log_requests` 表），
//! 合并旧 `log_metadata` 和 `log_token_usage` 的所有标量字段，
//! 一行对应一次完整的代理转发事件。作为 Dashboard / 列表 /
//! 详情 / 会话查询的唯一数据源，不再需要 LEFT JOIN。
//!
//! 数据分表原则：标量字段在此表（永久保留），大体积 JSON/TEXT
//! 放在 `log_contents` 表（按月分区、按 GB 上限清理）。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// SeaORM 实体映射 log_requests 表
///
/// 合并了旧 `log_metadata`（21 列）和 `log_token_usage`（25 列）的去重并集。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "log_requests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub timestamp: DateTimeWithTimeZone,
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    pub provider_id: Option<Uuid>,
    pub account_id: Option<Uuid>,

    // ─── 模型名称 ───
    pub model_original: Option<String>,
    pub model_mapped: Option<String>,
    /// 规范化模型名（trim + lowercase + 统一分隔符），用于 Dashboard 聚合
    pub model_normalized: String,

    // ─── 请求结果（来自旧 metadata）───
    pub status_code: Option<i16>,
    pub duration_ms: Option<i32>,
    pub error_message: Option<String>,
    pub is_interrupted: bool,
    pub has_error: bool,

    // ─── 协议与客户端（来自旧 metadata）───
    pub api_type: String,
    pub client_type: String,
    pub client_user_agent: Option<String>,
    pub client_version: Option<String>,

    // ─── 会话与会话内标识 ───
    pub conversation_source: String,
    pub agent_id: Option<String>,
    /// 代理类型（如 claude-code、sdk 等）
    pub agent_type: Option<String>,

    // ─── 词元用量（来自旧 token_usage）───
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_creation_input_tokens: i32,
    pub cache_read_input_tokens: i32,
    pub thinking_tokens: i32,
    pub total_tokens: i32,

    // ─── JSON 细节（来自旧 token_usage）───
    pub raw_usage: Option<Json>,
    pub server_tool_usage: Option<Json>,
    pub cache_creation: Option<Json>,

    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ─── 领域行为 ──────────────────────────────────────────────────────

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
