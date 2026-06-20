use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

/// 日志过滤参数
///
/// 用于日志列表和会话列表的分页查询过滤。
#[derive(Debug, Clone, Deserialize)]
pub struct LogFilterParams {
    /// 按会话 ID 过滤
    pub session_id: Option<String>,
    /// 按用户 ID 过滤
    pub user_id: Option<Uuid>,
    /// 按接入点 ID 过滤
    pub access_point_id: Option<Uuid>,
    /// 按开始时间过滤
    pub start_time: Option<DateTime<Utc>>,
    /// 按结束时间过滤
    pub end_time: Option<DateTime<Utc>>,
    /// 按状态码过滤
    pub status_code: Option<i16>,
    /// 按服务商 ID 过滤
    pub provider_id: Option<Uuid>,
    /// 按账号 ID 过滤
    pub account_id: Option<Uuid>,
    /// 按是否中断过滤（true = 中断，false = 未中断）
    pub is_interrupted: Option<bool>,
    /// 页码（从 1 开始）
    pub page: Option<u64>,
    /// 每页大小
    pub page_size: Option<u64>,
}
