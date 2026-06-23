//! 时间范围查询参数，作为 Dashboard 所有数据源的全局过滤器。

use chrono::{DateTime, Utc};
use serde::Deserialize;

/// 时间范围预设
#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeRangePreset {
    /// 今日（按小时分桶）
    Today,
    /// 近 7 天（按天分桶）
    #[serde(rename = "last7")]
    Last7,
    /// 近 30 天（按天分桶）
    #[serde(rename = "last30")]
    Last30,
    /// 自定义起止时间
    Custom,
}

/// Dashboard 时间范围查询参数
///
/// `start` 和 `end` 仅在 `range = Custom` 时使用；其它预设自动推算窗口。
#[derive(Debug, Clone, Deserialize)]
pub struct TimeRangeQuery {
    /// 时间范围预设
    pub range: TimeRangePreset,
    /// 自定义起始时间（ISO 8601），仅 Custom 模式必填
    pub start: Option<DateTime<Utc>>,
    /// 自定义结束时间（ISO 8601），仅 Custom 模式必填
    pub end: Option<DateTime<Utc>>,
}
