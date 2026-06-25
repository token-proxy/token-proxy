//! 审计日志筛选参数 DTO。

use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

/// 审计日志筛选参数（从 URL query string 反序列化）。
#[derive(Debug, Clone, Deserialize)]
pub struct AuditLogFilterParams {
    /// 操作类型，逗号分隔（如 "create,update"）
    pub action: Option<String>,
    /// 实体类型，逗号分隔
    pub entity_type: Option<String>,
    /// 操作者 ID
    pub operator_id: Option<Uuid>,
    /// 操作者类型（"user" 或 "system"）
    pub operator_type: Option<String>,
    /// 开始时间
    pub start_time: Option<DateTime<Utc>>,
    /// 结束时间
    pub end_time: Option<DateTime<Utc>>,
    /// 页码（从 1 开始）
    pub page: Option<u64>,
    /// 每页条数
    pub page_size: Option<u64>,
}
