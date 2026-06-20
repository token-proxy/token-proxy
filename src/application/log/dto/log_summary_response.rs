use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// 日志摘要响应（列表场景，不含请求/响应内容）
#[derive(Debug, Clone, Serialize)]
pub struct LogSummaryResponse {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    /// 实际使用的服务商 ID
    pub provider_id: Option<Uuid>,
    /// 实际使用的账号 ID
    pub account_id: Option<Uuid>,
    /// 原始请求模型
    pub model_original: Option<String>,
    /// 路由映射后的模型
    pub model_mapped: Option<String>,
    pub status_code: Option<i16>,
    /// 请求耗时（毫秒）
    pub duration_ms: Option<i32>,
    /// 客户端是否中途中断连接
    pub is_interrupted: bool,
    /// 会话来源（如 claude-code / claude-web）
    pub conversation_source: String,
    pub agent_id: Option<String>,
    pub client_name: Option<String>,
    pub client_version: Option<String>,
    pub client_channel: Option<String>,
    pub client_platform: Option<String>,
    pub api_type: String,
    /// Token 用量汇总
    pub token_input_tokens: Option<i32>,
    pub token_output_tokens: Option<i32>,
    pub token_cache_creation_input_tokens: Option<i32>,
    pub token_cache_read_input_tokens: Option<i32>,
    pub token_thinking_tokens: Option<i32>,
    pub token_total_tokens: Option<i32>,
}
