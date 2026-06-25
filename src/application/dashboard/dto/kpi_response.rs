//! Dashboard KPI 响应 DTO（含 sparkline 内嵌数据 + 词元构成 + 缓存命中率副产物）。

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

/// KPI 趋势项（适用于请求数 / 词元量等标量指标）
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

/// 比率趋势项
///
/// 用于成功率、缓存命中率等 `0.0 - 1.0` 比率指标。
/// 分母为 0 时 `rate = None`，前端显示 `—`。
#[derive(Debug, Clone, Serialize)]
pub struct RateTrendItem {
    /// 当前比率（0.0 - 1.0），None = 无样本
    pub rate: Option<f64>,
    /// 上一窗比率
    pub previous_rate: Option<f64>,
    /// 比率百分比变化
    pub change_pct: Option<f64>,
    /// 趋势徽章
    pub trend: TrendBadge,
}

/// 词元构成 5 维度（绝对值，前端自行计算百分比）
#[derive(Debug, Clone, Serialize)]
pub struct TokenComposition {
    /// 未命中缓存输入词元数
    pub input_tokens: i64,
    /// 输出词元数
    pub output_tokens: i64,
    /// 缓存创建输入词元数
    pub cache_creation_tokens: i64,
    /// 缓存命中输入词元数
    pub cache_read_tokens: i64,
    /// 思考词元数
    pub thinking_tokens: i64,
}

/// Sparkline 时间序列桶（多条序列共享时间轴）
#[derive(Debug, Clone, Serialize)]
pub struct SparklineBucketDto {
    /// 桶起始时间
    pub bucket_start: DateTime<Utc>,
    /// 该桶内请求总数
    pub request_count: i64,
    /// 该桶内词元总消耗
    pub total_tokens: i64,
}

/// Sparkline 序列容器
#[derive(Debug, Clone, Serialize)]
pub struct SparklineSeries {
    /// 时间序列桶（按 bucket_start 升序）
    pub buckets: Vec<SparklineBucketDto>,
}

/// Dashboard KPI 完整响应
///
/// 内嵌 sparkline 序列和缓存命中率，避免前端二次查询。
#[derive(Debug, Clone, Serialize)]
pub struct KpiResponse {
    /// 会话数 KPI（不重复 session_id 计数）
    pub session_count: KpiTrendItem,
    /// 请求数 KPI
    pub request_count: KpiTrendItem,
    /// 词元总量 KPI
    pub total_tokens: KpiTrendItem,
    /// 输入词元 KPI（input + cache_creation + cache_read 之和，含缓存词元）
    pub input_tokens: KpiTrendItem,
    /// 输出词元 KPI（output + thinking 之和，含思考词元）
    pub output_tokens: KpiTrendItem,
    /// 词元构成（5 维度绝对值）
    pub composition: TokenComposition,
    /// 缓存命中率（作为 KPI 端点副产物，避免二次查询）
    pub cache_hit_rate: CacheHitRate,
    /// 内嵌的时间序列数据（供 KPI 卡的 sparkline 使用）
    pub sparkline: SparklineSeries,
}
