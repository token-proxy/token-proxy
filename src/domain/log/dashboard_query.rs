//! Dashboard 聚合查询的领域读模型。
//!
//! 本模块定义 Dashboard 数据视图所需的中间数据结构（窗口、聚合行）。
//! 这些类型不属于业务实体，是 Repository 层为应用层 Service 提供的查询结果。
//!
//! ## 设计取舍
//!
//! Dashboard 是只读视图，需要在 SQL 层做 LEFT JOIN 容忍删除（access_points / providers）。
//! 因此 `name` / `short_code` 等字段都是 `Option<String>`，None 表示关联实体已删除。

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

/// Dashboard 时间窗口
///
/// 闭右开区间 `[start, end)`，由应用层 `time_window::resolve_windows` 解析得出。
#[derive(Debug, Clone)]
pub struct DashboardWindow {
    /// 起始时间（含）
    pub start: DateTime<Utc>,
    /// 结束时间（不含）
    pub end: DateTime<Utc>,
}

/// KPI 聚合结果
///
/// 单次 SQL 聚合返回的标量集合，覆盖请求数、会话数、各类词元维度。
#[derive(Debug, Clone)]
pub struct KpiAggregate {
    /// 总请求数
    pub request_count: i64,
    /// 不重复会话数（COUNT(DISTINCT session_id)）
    pub session_count: i64,
    /// 总词元数（来自 log_token_usage）
    pub total_tokens: i64,
    /// 未命中缓存输入词元数（input_tokens 列之和，不含缓存和思考）
    pub input_tokens: i64,
    /// 输出词元数（output_tokens 列之和，不含思考词元）
    pub output_tokens: i64,
    /// 缓存命中输入词元数（cache_read_input_tokens 列之和）
    pub cache_read_tokens: i64,
    /// 缓存创建输入词元数（cache_creation_input_tokens 列之和）
    pub cache_creation_tokens: i64,
    /// 思考词元数（thinking_tokens 列之和）
    pub thinking_tokens: i64,
    /// 输入方向词元总量（input + cache_creation + cache_read，缓存命中率的分母）
    pub total_input_side_tokens: i64,
}

/// Sparkline 时间序列桶
///
/// 请求与词元两条序列共享同一桶时间，便于前端共用 X 轴。
#[derive(Debug, Clone)]
pub struct SparklineBucket {
    /// 桶起始时间（已按 date_trunc 截断到 hour 或 day）
    pub bucket_start: DateTime<Utc>,
    /// 该桶内请求数
    pub request_count: i64,
    /// 该桶内词元总和
    pub total_tokens: i64,
}

/// 用量趋势时间序列桶
///
/// 按日展示请求数、会话数与 6 类词元，用于趋势图主数据源。
#[derive(Debug, Clone)]
pub struct UsageTrendBucket {
    /// 桶起始时间（按 UTC day 截断）
    pub bucket_start: DateTime<Utc>,
    /// 该桶内请求数
    pub request_count: i64,
    /// 该桶内不重复会话数
    pub session_count: i64,
    /// 该桶内总词元数
    pub total_tokens: i64,
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
    /// 该桶内按模型拆分的词元用量
    pub per_model: Vec<ModelTokenUsage>,
}

/// 单日单模型词元用量
///
/// 用于模型消费面积图的数据源，每个 bucket 包含若干模型的词元用量。
#[derive(Debug, Clone)]
pub struct ModelTokenUsage {
    /// 模型名（已归一化，来自 log_requests.model_normalized，不区分大小写和分隔符）
    pub model: String,
    /// 该模型在该桶内的总词元数
    pub total_tokens: i64,
}

/// 日历热力图单元格
///
/// 一天一格，记录该日总词元数与请求数；用于年度活跃度热力图渲染。
#[derive(Debug, Clone)]
pub struct HeatmapCell {
    /// 该单元格对应的日期（按 UTC 截断）
    pub day: NaiveDate,
    /// 该日总词元数
    pub total_tokens: i64,
    /// 该日请求数
    pub request_count: i64,
}

/// 模型排行行
///
/// 按模型名维度聚合窗口内的请求与词元消耗。
#[derive(Debug, Clone)]
pub struct TopModelRow {
    /// 模型名（来自 log_metadata.model）
    pub model: String,
    /// 窗口内请求数
    pub request_count: i64,
    /// 窗口内词元总消耗
    pub total_tokens: i64,
}

/// 接入点排行行
///
/// 按接入点维度聚合窗口内的请求与词元消耗；LEFT JOIN 容忍接入点删除。
#[derive(Debug, Clone)]
pub struct TopAccessPointRow {
    /// 接入点 UUID
    pub access_point_id: Uuid,
    /// 接入点名；None = access_points 表已无该记录（已被删除）
    pub name: Option<String>,
    /// 接入点短码；None 同上
    pub short_code: Option<String>,
    /// 窗口内请求数
    pub request_count: i64,
    /// 窗口内词元总消耗
    pub total_tokens: i64,
}

/// 服务质量指标
///
/// 汇总窗口内的请求成败分布与延迟分位数，用于成功率、错误率、p95 延迟卡片。
#[derive(Debug, Clone)]
pub struct QualityMetrics {
    /// 总请求数（成功 + 错误 + 中断）
    pub total_count: i64,
    /// 成功请求数（2xx）
    pub success_count: i64,
    /// 客户端错误数（4xx）
    pub client_error_count: i64,
    /// 服务端错误数（5xx）
    pub server_error_count: i64,
    /// 中断请求数（is_interrupted = true）
    pub interrupted_count: i64,
    /// 平均时延（毫秒）；样本为空时为 None
    pub avg_duration_ms: Option<f64>,
    /// p95 时延（毫秒）；样本为空时为 None
    pub p95_duration_ms: Option<f64>,
}
