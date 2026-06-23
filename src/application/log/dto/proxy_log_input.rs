use axum::http::HeaderMap;
use chrono::{DateTime, FixedOffset};
use uuid::Uuid;

/// 代理日志输入参数 — `LogService::record_proxy_log` 的入参契约
///
/// 一次性构造、无占位值的领域语义参数集合。
/// 由 `ProxyCallRecord::finish`（应用层）在代理调用生命周期结束时构造，
/// 包含一次代理转发产生的全部已知信息（请求头体、响应头体、耗时、中断标记等）。
///
/// `LogService` 拿到后内部构造 `LogMetadata` / `LogContent` / `LogTokenUsage` 三个领域实体并落库。
#[derive(Clone)]
pub struct ProxyLogInput {
    /// 请求时间戳（构造时一次性生成，metadata 和 contents 共用）
    pub timestamp: DateTime<FixedOffset>,
    /// 客户端会话标识；`None` 表示请求未携带会话标识（如非 Claude Code 客户端的直接调用）
    pub session_id: Option<String>,
    pub user_id: Uuid,
    pub access_point_id: Uuid,
    pub provider_id: Uuid,
    pub account_id: Uuid,
    pub model_original: String,
    pub model_mapped: String,
    pub api_type: String,
    pub status_code: u16,
    pub request_headers: HeaderMap,
    pub request_body: serde_json::Value,
    pub resp_headers: HeaderMap,
    pub response_body: String,
    pub duration_ms: i32,
    /// 客户端是否在响应中途断开
    pub is_interrupted: bool,
    pub error_message: Option<String>,
}
