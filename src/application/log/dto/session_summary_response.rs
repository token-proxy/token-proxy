use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// 会话摘要响应（列表场景）
///
/// 包含会话的基本信息和累计 token 用量。
#[derive(Debug, Clone, Serialize)]
pub struct SessionSummaryResponse {
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    /// 会话开始时间
    pub start_time: DateTime<Utc>,
    /// 会话中的请求总数
    pub request_count: u64,
    /// 累计 token 用量
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_creation_input_tokens: i64,
    pub total_cache_read_input_tokens: i64,
    pub total_thinking_tokens: i64,
    pub total_tokens: i64,
}
