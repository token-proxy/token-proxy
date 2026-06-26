//! 时间范围查询参数，作为 Dashboard 所有数据源的全局过滤器。

use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Dashboard 时间范围参数
///
/// 所有端点统一接收具体起止时刻。预设计算移到前端，后端仅负责窗口推导与分桶。
/// `tz` 为 IANA 时区名（如 `Asia/Shanghai`），用于分桶函数的 `date_trunc AT TIME ZONE`；
/// 不传时默认 UTC。
#[derive(Debug, Clone, Deserialize)]
pub struct TimeRangeParams {
    /// 起始时刻（ISO 8601）
    pub start: DateTime<Utc>,
    /// 结束时刻（ISO 8601，不含）
    pub end: DateTime<Utc>,
    /// IANA 时区名，默认 UTC
    #[serde(default = "default_tz")]
    pub tz: String,
}

fn default_tz() -> String {
    "UTC".to_string()
}
