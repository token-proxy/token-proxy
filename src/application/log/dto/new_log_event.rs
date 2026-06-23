//! 新日志事件 DTO — SSE 广播事件负载
//!
//! 不含敏感数据（request_body、response_body、request_headers 等），
//! 仅包含足以让前端判断刷新行为的标识信息。

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// 新日志写入成功的广播事件
///
/// 每次 `LogService::record_proxy_log` 成功完成后通过 broadcast channel 发送。
/// 负载不含敏感数据，仅用作前端自动刷新的触发信号。
#[derive(Clone, Debug, Serialize)]
pub struct NewLogEvent {
    /// 日志 ID
    pub log_id: Uuid,
    /// 写入时间戳
    pub timestamp: DateTime<Utc>,
    /// 会话 ID（"unknown" 代表无会话标识）
    pub session_id: String,
    /// API 类型（如 "anthropic"、"openai"）
    pub api_type: String,
    /// 用户 ID
    pub user_id: Uuid,
    /// 接入点 ID
    pub access_point_id: Uuid,
}
