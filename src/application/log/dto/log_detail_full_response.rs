use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// 日志完整详情响应（含客户端信息、词元用量等）
#[derive(Debug, Clone, Serialize)]
pub struct LogDetailFullResponse {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub access_point_id: Option<Uuid>,
    pub access_point_name: Option<String>,
    pub provider_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub model_original: String,
    pub model_mapped: String,
    pub status_code: i32,
    pub duration_ms: i32,
    pub error_message: Option<String>,
    pub conversation_source: String,
    pub agent_id: Option<String>,
    /// 客户端版本号（从 User-Agent 解析的版本号段）
    pub client_version: Option<String>,
    pub api_type: Option<String>,
    /// 本次请求实际使用的协议（anthropic / openai / openai_response）
    pub api_protocol: Option<String>,
    /// 原始内容是否落库（false = `log_contents` 无对应行，请求/响应原文已不可用）
    ///
    /// 内容按月分区存放，分区被清理或数据库重建后历史行会缺失原文，
    /// 而 `log_requests` 长期保留——前端据此区分「未保存原文」与「解析失败」。
    pub content_available: bool,
    pub request_headers: serde_json::Value,
    pub response_headers: serde_json::Value,
    pub request_body: serde_json::Value,
    pub response_body: String,
    pub token_input_tokens: Option<i32>,
    pub token_output_tokens: Option<i32>,
    pub token_cache_creation_input_tokens: Option<i32>,
    pub token_cache_read_input_tokens: Option<i32>,
    pub token_thinking_tokens: Option<i32>,
    pub token_total_tokens: Option<i32>,
    pub token_raw_usage: Option<serde_json::Value>,
}
