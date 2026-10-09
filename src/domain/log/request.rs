//! 代理请求实体 — domain/log/
//!
//! 定义 `LogRequest`（SeaORM 实体映射 `log_requests` 表），
//! 承载一次完整代理转发事件的全部标量字段与词元用量，
//! 一行对应一次转发。作为 Dashboard / 列表 / 详情 / 会话查询的
//! 唯一数据源，无需 LEFT JOIN（历史表已在迁移 `m20260628_000005` 中合并删除）。
//!
//! 数据分表原则：标量字段在此表（永久保留），大体积 JSON/TEXT
//! 放在 `log_contents` 表（按月分区、按 GB 上限清理）。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// SeaORM 实体映射 log_requests 表
///
/// 由历史表（已于 `m20260628_000005` 合并删除）的去重并集演化而来，
/// 是标量字段与词元用量的唯一落库位置。
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
    /// 接入点类型（协议家族）：anthropic / openai
    pub api_type: String,
    /// 本次请求实际使用的协议：anthropic / openai / openai_response
    pub api_protocol: String,
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
