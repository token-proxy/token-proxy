//! Dashboard KPI 响应 DTO（含三条 sparkline 内嵌数据）。

use chrono::{DateTime, Utc};
use serde::Serialize;

/// 趋势徽章
///
/// - `Up` / `Down` / `Flat`：有上一窗对比数据时的常规趋势
/// - `New`：当前 > 0 且上一窗 = 0（无法计算百分比）
/// - `Empty`：双窗皆为 0（无趋势可言）
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendBadge {
    Up,
    Down,
    Flat,
    New,
    Empty,
}

/// KPI 趋势项（适用于请求数 / Token 量 / 活跃成员数）
#[derive(Debug, Clone, Serialize)]
pub struct KpiTrendItem {
    /// 当前窗口值
    pub current: i64,
    /// 上一等长窗口值
    pub previous: i64,
    /// 趋势徽章
    pub trend: TrendBadge,
    /// 百分比变化（如 +12.3 表示上升 12.3%）；None 表示无法计算（Empty 或 New）
    pub change_pct: Option<f64>,
}

/// 缓存命中率
///
/// 分母为 0 时 `rate = None`，前端显示 `—`。
#[derive(Debug, Clone, Serialize)]
pub struct CacheHitRate {
    /// 当前命中率（0.0 - 1.0），None = 无可命中数据
    pub rate: Option<f64>,
    /// 上一窗命中率
    pub previous_rate: Option<f64>,
    /// 命中率百分比变化
    pub change_pct: Option<f64>,
    /// 趋势徽章
    pub trend: TrendBadge,
}

/// Sparkline 时间序列桶（三条序列共享时间轴）
#[derive(Debug, Clone, Serialize)]
pub struct SparklineBucketDto {
    /// 桶起始时间
    pub bucket_start: DateTime<Utc>,
    /// 该桶内请求总数
    pub request_count: i64,
    /// 该桶内 token 总消耗
    pub total_tokens: i64,
    /// 该桶内去重活跃成员数
    pub active_user_count: i64,
}

/// Sparkline 序列容器
#[derive(Debug, Clone, Serialize)]
pub struct SparklineSeries {
    /// 时间序列桶（按 bucket_start 升序）
    pub buckets: Vec<SparklineBucketDto>,
}

/// Dashboard KPI 完整响应（4 张卡 + sparkline 序列）
#[derive(Debug, Clone, Serialize)]
pub struct KpiResponse {
    /// 请求数 KPI
    pub request_count: KpiTrendItem,
    /// Token 总量 KPI
    pub total_tokens: KpiTrendItem,
    /// 活跃成员数 KPI
    pub active_user_count: KpiTrendItem,
    /// 缓存命中率 KPI
    pub cache_hit_rate: CacheHitRate,
    /// 内嵌的时间序列数据（供前 3 张 KPI 卡的 sparkline 使用）
    pub sparkline: SparklineSeries,
}
