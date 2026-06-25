//! Dashboard 调用质量响应 DTO。

use serde::Serialize;

use crate::application::dashboard::dto::RateTrendItem;

/// 调用质量指标
///
/// 所有 `*_rate` 字段在 `total_count == 0` 时为 None（前端显示 `—`）。
/// 时延字段在无样本时同样为 None。
#[derive(Debug, Clone, Serialize)]
pub struct QualityResponse {
    /// 窗口内总请求数（速率分母）
    pub total_count: i64,
    /// 成功率趋势（2xx 占比，0.0 - 1.0）
    pub success_rate: RateTrendItem,
    /// 客户端错误率（4xx 占比）
    pub client_error_rate: Option<f64>,
    /// 服务端错误率（5xx 占比）
    pub server_error_rate: Option<f64>,
    /// 中断率（客户端断开或 SSE 截断）
    pub interrupted_rate: Option<f64>,
    /// 平均时延（毫秒）
    pub avg_duration_ms: Option<f64>,
    /// p95 时延（毫秒）
    pub p95_duration_ms: Option<f64>,
}
