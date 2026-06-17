use axum::http::HeaderMap;
use chrono::{DateTime, FixedOffset};
use uuid::Uuid;

/// 代理日志数据 DTO — ProxyLogger → LogService 之间的数据契约
///
/// ProxyLogger 在整个代理生命周期中逐步填充此 DTO 的字段，
/// flush 时一次性交给 LogService::record_proxy_log()。
/// LogService 拿到后自行构造 LogMetadata / LogContent / LogTokenUsage。
#[derive(Clone)]
pub struct ProxyLogData {
    // 请求时间戳（构造时一次性生成，metadata 和 contents 共用）
    pub timestamp: DateTime<FixedOffset>,
    // 请求标识
    pub session_id: String,
    pub user_id: Uuid,
    pub access_point_id: Uuid,
    pub provider_id: Uuid,
    pub account_id: Uuid,
    pub model_original: String,
    pub model_mapped: String,
    pub api_type: String,
    pub status_code: u16,
    // 原始请求数据
    pub request_headers: HeaderMap,
    pub request_body: serde_json::Value,
    // 原始响应数据
    pub resp_headers: HeaderMap,
    pub response_body: String,
    // 生命周期标记
    pub duration_ms: i32,
    pub is_interrupted: bool,
    pub error_message: Option<String>,
}
