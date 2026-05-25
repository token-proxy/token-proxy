use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 日志条目实体，记录每次代理请求的元数据和可展示摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
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
    /// 解析器版本号
    pub parser_version: Option<String>,
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

/// 日志内容实体，记录请求和响应的完整数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogContent {
    pub log_id: Uuid,
    pub request_headers: serde_json::Value,
    pub request_body: serde_json::Value,
    pub response_body: String,
}

impl Default for LogEntry {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            session_id: String::new(),
            user_id: None,
            access_point_id: None,
            provider_id: None,
            account_id: None,
            model_original: None,
            model_mapped: None,
            status_code: None,
            duration_ms: None,
            error_message: None,
            request_index: 0,
            client_session_id: None,
            client_app: None,
            client_user_agent: None,
            conversation_source: "unknown".to_string(),
            agent_id: None,
            agent_type: None,
            parent_agent_tool_use_id: None,
            request_kind: None,
            primary_tool_name: None,
            message_preview: None,
            message_full: None,
            response_preview: None,
            has_thinking: false,
            has_tool_use: false,
            has_error: false,
            raw_content_available: true,
            parser_version: None,
            client_name: None,
            client_version: None,
            client_channel: None,
            client_platform: None,
            api_type: "anthropic".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConversationEvent {
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
    pub created_at: DateTime<Utc>,
    /// content block 子类型: text/thinking/redacted_thinking/tool_use/tool_result/server_tool_use
    pub content_type: Option<String>,
    /// redacted_thinking 的签名
    pub signature: Option<String>,
    /// tool_result 的完整内容文本
    pub tool_result_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogTokenUsage {
    pub id: Uuid,
    pub log_id: Uuid,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
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
    pub raw_usage: Option<serde_json::Value>,
    /// 服务端工具用量（JSONB: web_search_requests, web_fetch_requests）
    pub server_tool_usage: Option<serde_json::Value>,
    /// 缓存创建详情（JSONB: ephemeral_5m_input_tokens, ephemeral_1h_input_tokens）
    pub cache_creation: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}
