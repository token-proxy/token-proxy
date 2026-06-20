use serde::{Deserialize, Serialize};

/// 系统设置响应体
#[derive(Debug, Clone, Serialize)]
pub struct SettingsResponse {
    /// 日志保留月数
    pub log_retention_months: i16,
}

/// 更新系统设置请求体
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSettingsRequest {
    /// 日志保留月数（取值范围 1-36）
    pub log_retention_months: i16,
}
