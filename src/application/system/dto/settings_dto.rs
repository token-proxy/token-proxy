use serde::{Deserialize, Serialize};

/// 系统设置响应体
#[derive(Debug, Clone, Serialize)]
pub struct SettingsResponse {
    /// 日志保留月数（1-36）
    pub log_retention_months: i16,
    /// 日志占用上限（GiB），None 表示不限制
    pub log_storage_cap_gb: Option<i16>,
    /// 当前有日志数据的月份数
    pub log_month_count: usize,
    /// 日志总磁盘占用（字节），前端按 1024 进制格式化为 GiB
    pub total_size_bytes: i64,
}

/// 更新系统设置请求体
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSettingsRequest {
    /// 日志保留月数（取值范围 1-36）
    pub log_retention_months: i16,
    /// 日志占用上限（GiB），None 表示不限制（清空上限传 null）
    pub log_storage_cap_gb: Option<i16>,
}
