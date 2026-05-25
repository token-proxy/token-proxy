use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Request / Query DTOs ───

#[derive(Debug, Clone, Deserialize)]
pub struct LogFilterParams {
    pub session_id: Option<String>,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub status_code: Option<i16>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// ─── Response DTOs ───

#[derive(Debug, Clone, Serialize)]
pub struct LogSummaryResponse {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    pub model_original: Option<String>,
    pub model_mapped: Option<String>,
    pub status_code: Option<i16>,
    pub duration_ms: Option<i32>,
    pub conversation_source: String,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    pub request_kind: Option<String>,
    pub primary_tool_name: Option<String>,
    pub message_preview: Option<String>,
    pub message_full: Option<String>,
    pub response_preview: Option<String>,
    pub has_thinking: bool,
    pub has_tool_use: bool,
    pub raw_content_available: bool,
    /// 解析器版本号
    pub parser_version: Option<String>,
    /// 客户端名称
    pub client_name: Option<String>,
    /// 客户端版本号
    pub client_version: Option<String>,
    /// 客户端发布渠道
    pub client_channel: Option<String>,
    /// 客户端平台
    pub client_platform: Option<String>,
    /// API 类型
    pub api_type: String,
    /// token 输入用量
    pub token_input_tokens: Option<i32>,
    /// token 输出用量
    pub token_output_tokens: Option<i32>,
    /// token 总用量
    pub token_total_tokens: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogDetailResponse {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
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
    pub request_headers: Option<serde_json::Value>,
    pub request_body: Option<serde_json::Value>,
    pub response_body: Option<String>,
}

/// 日志完整详情响应（包含客户端信息、token 用量等）
#[derive(Debug, Clone, Serialize)]
pub struct LogDetailFullResponse {
    /// 基础信息
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    pub provider_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub model_original: String,
    pub model_mapped: String,
    pub status_code: i32,
    pub duration_ms: i32,
    pub error_message: Option<String>,
    pub request_index: i32,
    pub conversation_source: String,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    /// 客户端信息
    pub parser_version: Option<String>,
    pub client_name: Option<String>,
    pub client_version: Option<String>,
    pub client_channel: Option<String>,
    pub client_platform: Option<String>,
    /// 请求内容
    pub request_headers: serde_json::Value,
    pub request_body: serde_json::Value,
    /// 最后一条 user message 的文本
    pub request_message_text: Option<String>,
    /// 响应内容
    pub response_body: String,
    pub response_assistant_text: Option<String>,
    pub response_thinking_text: Option<String>,
    /// Token 用量
    pub token_input_tokens: Option<i32>,
    pub token_output_tokens: Option<i32>,
    pub token_cache_creation_input_tokens: Option<i32>,
    pub token_cache_read_input_tokens: Option<i32>,
    pub token_thinking_tokens: Option<i32>,
    pub token_total_tokens: Option<i32>,
    pub token_raw_usage: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionSummaryResponse {
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    pub start_time: DateTime<Utc>,
    pub request_count: u64,
    pub first_message: Option<String>,
    /// 总输入 token 数
    pub total_input_tokens: i64,
    /// 总输出 token 数
    pub total_output_tokens: i64,
    /// 总缓存创建输入 token 数
    pub total_cache_creation_input_tokens: i64,
    /// 总缓存读取输入 token 数
    pub total_cache_read_input_tokens: i64,
    /// 总 thinking token 数
    pub total_thinking_tokens: i64,
    /// 总 token 数
    pub total_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConversationEventResponse {
    pub id: Uuid,
    pub log_id: Uuid,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
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
    pub hidden_content: Option<serde_json::Value>,
    pub display_payload: Option<serde_json::Value>,
    pub confidence: i16,
    /// content block 子类型
    pub content_type: Option<String>,
    /// redacted_thinking 签名
    pub signature: Option<String>,
    /// tool_result 内容
    pub tool_result_content: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenUsageResponse {
    pub id: Uuid,
    pub log_id: Uuid,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_creation_input_tokens: i32,
    pub cache_read_input_tokens: i32,
    pub thinking_tokens: i32,
    pub total_tokens: i32,
    pub raw_usage: Option<serde_json::Value>,
    /// 服务端工具用量
    pub server_tool_usage: Option<serde_json::Value>,
    /// 缓存创建详情
    pub cache_creation: Option<serde_json::Value>,
}
