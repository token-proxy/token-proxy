//! Dashboard 应用层 Service：编排个人视角的 KPI / 热力图 / 排行 / 质量查询。
//!
//! ## 职责
//!
//! - 解析 `TimeRangeQuery` 为当窗 + 上一窗（委托 `time_window::resolve_windows`）
//! - 校验热力图时区参数（委托 `timezone::validate_timezone`）
//! - 调度 `LogRepository` 的用户视角聚合方法
//! - 计算 KPI 趋势徽章（`compute_trend` 纯函数）和缓存命中率
//! - 装配 DTO 返回给展示层
//!
//! ## 设计纪律
//!
//! 本 Service 仅依赖 `LogRepository`（事实表入口），所有跨表 LEFT JOIN
//! 在 Repository 层完成，避免 N+1 查询和跨聚合依赖。
//! 所有方法首参为 `user_id: Uuid`，即当前登录用户视角。

use std::sync::Arc;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::application::dashboard::dto::{
    CacheHitRate, HeatmapCellDto, HeatmapResponse, KpiResponse, KpiTrendItem, QualityResponse,
    RateTrendItem, SparklineBucketDto, SparklineSeries, TimeRangePreset, TimeRangeQuery,
    TokenComposition, TopAccessPointItem, TopAccessPointsResponse, TopModelItem, TopModelsResponse,
    TrendBadge, UsageTrendBucketDto, UsageTrendsResponse,
};
use crate::application::dashboard::time_window::resolve_windows;
use crate::application::dashboard::timezone::validate_timezone;
use crate::domain::log::dashboard_query::KpiAggregate;
use crate::domain::log::repository_log::LogRepository;
use crate::shared::error::AppError;

/// 模型排行 Top N 上限
const RANKING_LIMIT_MODELS: u32 = 8;

/// 接入点排行 Top N 上限
const RANKING_LIMIT_ACCESS_POINTS: u32 = 5;

/// 趋势"持平"判定阈值（绝对百分比 < FLAT_THRESHOLD 视为持平）
const FLAT_THRESHOLD: f64 = 0.5;

/// 自定义用量趋势最大跨度天数
const USAGE_TRENDS_CUSTOM_MAX_DAYS: i64 = 366;

/// Dashboard 应用层 Service
pub struct DashboardService {
    log_repository: Arc<dyn LogRepository>,
}

impl DashboardService {
    /// 构造 DashboardService
    pub fn new(log_repository: Arc<dyn LogRepository>) -> Self {
        Self { log_repository }
    }

    /// 获取 KPI 卡片 + 词元构成 + 缓存命中率 + 内嵌 sparkline
    ///
    /// 数据按 `user_id` 维度过滤，仅展示当前登录用户的请求与词元数据。
    #[tracing::instrument(skip_all, fields(user_id = %user_id, range = ?q.range))]
    pub async fn get_kpi(&self, user_id: Uuid, q: TimeRangeQuery) -> Result<KpiResponse, AppError> {
        let ranges = resolve_windows(&q)?;

        // 1. 并行聚合：当窗 KPI、上一窗 KPI、当窗 sparkline
        let (current, previous, sparkline) = tokio::try_join!(
            self.log_repository.aggregate_kpi(user_id, &ranges.current),
            self.log_repository.aggregate_kpi(user_id, &ranges.previous),
            self.log_repository
                .aggregate_sparkline(user_id, &ranges.current, ranges.bucket_count),
        )?;

        // 2. 装配 KPI 标量项
        // ─── 注意：输入词元 = 未命中缓存输入 + 缓存创建输入 + 缓存命中输入（用户体验视角的"输入总量"）───
        // ─── 注意：输出词元 = 输出 + 思考词元（思考也是输出方向消耗）───
        let total_input_tokens =
            current.input_tokens + current.cache_creation_tokens + current.cache_read_tokens;
        let prev_total_input_tokens =
            previous.input_tokens + previous.cache_creation_tokens + previous.cache_read_tokens;
        let total_output_tokens = current.output_tokens + current.thinking_tokens;
        let prev_total_output_tokens = previous.output_tokens + previous.thinking_tokens;

        let session_count = build_trend_item(current.session_count, previous.session_count);
        let request_count = build_trend_item(current.request_count, previous.request_count);
        let total_tokens = build_trend_item(current.total_tokens, previous.total_tokens);
        let input_tokens = build_trend_item(total_input_tokens, prev_total_input_tokens);
        let output_tokens = build_trend_item(total_output_tokens, prev_total_output_tokens);
        let cache_hit_rate = build_cache_hit_rate(&current, &previous);

        // 3. 装配词元构成（5 维度绝对值）
        let composition = TokenComposition {
            input_tokens: current.input_tokens,
            output_tokens: current.output_tokens,
            cache_creation_tokens: current.cache_creation_tokens,
            cache_read_tokens: current.cache_read_tokens,
            thinking_tokens: current.thinking_tokens,
        };

        // 4. 转换 sparkline 桶为 DTO
        let buckets: Vec<SparklineBucketDto> = sparkline
            .into_iter()
            .map(|b| SparklineBucketDto {
                bucket_start: b.bucket_start,
                request_count: b.request_count,
                total_tokens: b.total_tokens,
            })
            .collect();

        Ok(KpiResponse {
            session_count,
            request_count,
            total_tokens,
            input_tokens,
            output_tokens,
            composition,
            cache_hit_rate,
            sparkline: SparklineSeries { buckets },
        })
    }

    /// 获取用量趋势（日级请求数与词元分项）
    ///
    /// 仅支持 `last30` 和 `custom`，避免短窗口与 KPI sparkline 语义重叠。
    /// 自定义范围最大跨度为 366 天，防止趋势查询一次扫描过多分区。
    #[tracing::instrument(skip_all, fields(user_id = %user_id, range = ?q.range))]
    pub async fn get_usage_trends(
        &self,
        user_id: Uuid,
        q: TimeRangeQuery,
    ) -> Result<UsageTrendsResponse, AppError> {
        match q.range {
            TimeRangePreset::Last30 | TimeRangePreset::Custom => {}
            TimeRangePreset::Today | TimeRangePreset::Last7 => {
                return Err(AppError::Validation(
                    "用量趋势仅支持 last30 或 custom 时间范围".to_string(),
                ));
            }
        }

        let window = resolve_windows(&q)?.current;
        if matches!(q.range, TimeRangePreset::Custom)
            && window.end - window.start > Duration::days(USAGE_TRENDS_CUSTOM_MAX_DAYS)
        {
            return Err(AppError::Validation(format!(
                "用量趋势自定义时间范围不能超过 {} 天",
                USAGE_TRENDS_CUSTOM_MAX_DAYS
            )));
        }

        let buckets = self
            .log_repository
            .usage_trends_for_user(user_id, &window)
            .await?;

        Ok(UsageTrendsResponse {
            buckets: buckets
                .into_iter()
                .map(|b| UsageTrendBucketDto {
                    bucket_start: b.bucket_start,
                    request_count: b.request_count,
                    session_count: b.session_count,
                    total_tokens: b.total_tokens,
                    input_tokens: b.input_tokens,
                    output_tokens: b.output_tokens,
                    cache_creation_tokens: b.cache_creation_tokens,
                    cache_read_tokens: b.cache_read_tokens,
                    thinking_tokens: b.thinking_tokens,
                })
                .collect(),
        })
    }

    /// 获取近 1 年（365 天）词元用量日级热力图
    ///
    /// `tz` 必须是合法 IANA 时区名（如 `Asia/Shanghai`），用于将 UTC 时间戳分桶到用户本地日期。
    /// 校验失败时返回 `AppError::Validation`。
    #[tracing::instrument(skip_all, fields(user_id = %user_id, timezone = %tz))]
    pub async fn get_heatmap(&self, user_id: Uuid, tz: &str) -> Result<HeatmapResponse, AppError> {
        let validated_tz = validate_timezone(tz)?;
        let cells = self
            .log_repository
            .user_daily_token_heatmap(user_id, Utc::now(), &validated_tz)
            .await?;
        Ok(HeatmapResponse {
            cells: cells
                .into_iter()
                .map(|c| HeatmapCellDto {
                    day: c.day,
                    total_tokens: c.total_tokens,
                    request_count: c.request_count,
                })
                .collect(),
        })
    }

    /// 获取模型词元消耗排行 Top 8（用户视角）
    #[tracing::instrument(skip_all, fields(user_id = %user_id, range = ?q.range))]
    pub async fn get_top_models(
        &self,
        user_id: Uuid,
        q: TimeRangeQuery,
    ) -> Result<TopModelsResponse, AppError> {
        let window = resolve_windows(&q)?.current;
        let rows = self
            .log_repository
            .top_models_for_user(user_id, &window, RANKING_LIMIT_MODELS)
            .await?;
        Ok(TopModelsResponse {
            items: rows
                .into_iter()
                .map(|r| TopModelItem {
                    model: r.model,
                    request_count: r.request_count,
                    total_tokens: r.total_tokens,
                })
                .collect(),
        })
    }

    /// 获取接入点词元消耗排行 Top 5（用户视角，LEFT JOIN 容忍接入点删除）
    #[tracing::instrument(skip_all, fields(user_id = %user_id, range = ?q.range))]
    pub async fn get_top_access_points(
        &self,
        user_id: Uuid,
        q: TimeRangeQuery,
    ) -> Result<TopAccessPointsResponse, AppError> {
        let window = resolve_windows(&q)?.current;
        let rows = self
            .log_repository
            .top_access_points_for_user(user_id, &window, RANKING_LIMIT_ACCESS_POINTS)
            .await?;
        Ok(TopAccessPointsResponse {
            items: rows
                .into_iter()
                .map(|r| TopAccessPointItem {
                    access_point_id: r.access_point_id,
                    name: r.name,
                    short_code: r.short_code,
                    request_count: r.request_count,
                    total_tokens: r.total_tokens,
                })
                .collect(),
        })
    }

    /// 获取调用质量指标（成功率 / 错误率 / 中断率 / 平均与 p95 时延）
    ///
    /// `total_count == 0` 时所有 `*_rate` 字段返回 None（前端显示 `—`）。
    #[tracing::instrument(skip_all, fields(user_id = %user_id, range = ?q.range))]
    pub async fn get_quality(
        &self,
        user_id: Uuid,
        q: TimeRangeQuery,
    ) -> Result<QualityResponse, AppError> {
        let ranges = resolve_windows(&q)?;
        let (current, previous) = tokio::try_join!(
            self.log_repository
                .quality_metrics_for_user(user_id, &ranges.current),
            self.log_repository
                .quality_metrics_for_user(user_id, &ranges.previous),
        )?;
        // 样本数为 0 时分子分母均无意义，所有比率统一返回 None
        let total = current.total_count;
        let rate = |count: i64| -> Option<f64> {
            if total == 0 {
                None
            } else {
                Some(count as f64 / total as f64)
            }
        };
        Ok(QualityResponse {
            total_count: total,
            success_rate: build_rate_trend_item(
                current.success_count,
                current.total_count,
                previous.success_count,
                previous.total_count,
            ),
            client_error_rate: rate(current.client_error_count),
            server_error_rate: rate(current.server_error_count),
            interrupted_rate: rate(current.interrupted_count),
            avg_duration_ms: current.avg_duration_ms,
            p95_duration_ms: current.p95_duration_ms,
        })
    }
}

/// 计算趋势徽章 + 百分比变化
///
/// 5 种边界：
/// 1. 双 0 —— `Empty / None`
/// 2. 当 > 0、上一 = 0 —— `New / None`（无法计算百分比）
/// 3. 当 = 0、上一 > 0 —— `Down / Some(-100.0)`
/// 4. 绝对百分比 < `FLAT_THRESHOLD` —— `Flat / Some(pct)`
/// 5. 其它 —— `Up` 或 `Down / Some(pct)`
fn compute_trend(current: i64, previous: i64) -> (TrendBadge, Option<f64>) {
    if current == 0 && previous == 0 {
        return (TrendBadge::Empty, None);
    }
    if previous == 0 {
        return (TrendBadge::New, None);
    }
    if current == 0 {
        return (TrendBadge::Down, Some(-100.0));
    }
    let pct = ((current - previous) as f64 / previous as f64) * 100.0;
    if pct.abs() < FLAT_THRESHOLD {
        (TrendBadge::Flat, Some(pct))
    } else if pct > 0.0 {
        (TrendBadge::Up, Some(pct))
    } else {
        (TrendBadge::Down, Some(pct))
    }
}

/// 同 `compute_trend`，但接受 `Option<f64>` 比率
///
/// 用于缓存命中率（分母为 0 时整体为 None）的趋势判定。
fn compute_rate_trend(
    current_rate: Option<f64>,
    previous_rate: Option<f64>,
) -> (TrendBadge, Option<f64>) {
    match (current_rate, previous_rate) {
        (None, None) => (TrendBadge::Empty, None),
        (Some(_), None) => (TrendBadge::New, None),
        (None, Some(_)) => (TrendBadge::Down, Some(-100.0)),
        (Some(c), Some(0.0)) => {
            if c == 0.0 {
                (TrendBadge::Empty, None)
            } else {
                (TrendBadge::New, None)
            }
        }
        (Some(c), Some(p)) => {
            let pct = ((c - p) / p) * 100.0;
            if pct.abs() < FLAT_THRESHOLD {
                (TrendBadge::Flat, Some(pct))
            } else if pct > 0.0 {
                (TrendBadge::Up, Some(pct))
            } else {
                (TrendBadge::Down, Some(pct))
            }
        }
    }
}

/// 装配单个 KPI 趋势项
fn build_trend_item(current: i64, previous: i64) -> KpiTrendItem {
    let (trend, change_pct) = compute_trend(current, previous);
    KpiTrendItem {
        current,
        previous,
        trend,
        change_pct,
    }
}

/// 装配缓存命中率项
fn build_cache_hit_rate(current: &KpiAggregate, previous: &KpiAggregate) -> CacheHitRate {
    let rate = compute_rate(current.cache_read_tokens, current.total_input_side_tokens);
    let previous_rate = compute_rate(previous.cache_read_tokens, previous.total_input_side_tokens);
    let (trend, change_pct) = compute_rate_trend(rate, previous_rate);
    CacheHitRate {
        rate,
        previous_rate,
        change_pct,
        trend,
    }
}

/// 装配通用比率趋势项
fn build_rate_trend_item(
    current_numerator: i64,
    current_denominator: i64,
    previous_numerator: i64,
    previous_denominator: i64,
) -> RateTrendItem {
    let rate = compute_rate(current_numerator, current_denominator);
    let previous_rate = compute_rate(previous_numerator, previous_denominator);
    let (trend, change_pct) = compute_rate_trend(rate, previous_rate);
    RateTrendItem {
        rate,
        previous_rate,
        change_pct,
        trend,
    }
}

/// 计算缓存命中率：`cache_read / (input + cache_creation + cache_read)`；分母为 0 返回 None
fn compute_rate(numerator: i64, denominator: i64) -> Option<f64> {
    if denominator <= 0 {
        None
    } else {
        Some(numerator as f64 / denominator as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_trend_empty_when_both_zero() {
        assert!(matches!(compute_trend(0, 0), (TrendBadge::Empty, None)));
    }

    #[test]
    fn compute_trend_new_when_previous_zero_current_positive() {
        assert!(matches!(compute_trend(100, 0), (TrendBadge::New, None)));
    }

    #[test]
    fn compute_trend_down_minus_hundred_when_current_zero() {
        let (badge, pct) = compute_trend(0, 100);
        assert!(matches!(badge, TrendBadge::Down));
        assert_eq!(pct, Some(-100.0));
    }

    #[test]
    fn compute_trend_up_when_current_greater() {
        let (badge, pct) = compute_trend(120, 100);
        assert!(matches!(badge, TrendBadge::Up));
        match pct {
            Some(p) => assert!((p - 20.0).abs() < 0.001),
            None => panic!("expected Some(20.0)"),
        }
    }

    #[test]
    fn compute_trend_flat_within_threshold() {
        // 1000 → 1004 → 0.4%，小于 FLAT_THRESHOLD = 0.5
        let (badge, _) = compute_trend(1004, 1000);
        assert!(matches!(badge, TrendBadge::Flat));
    }

    #[test]
    fn build_rate_trend_item_empty_when_both_denominators_zero() {
        let item = build_rate_trend_item(0, 0, 0, 0);
        assert!(item.rate.is_none());
        assert!(item.previous_rate.is_none());
        assert!(matches!(item.trend, TrendBadge::Empty));
        assert!(item.change_pct.is_none());
    }

    #[test]
    fn build_rate_trend_item_up_when_rate_increases() {
        let item = build_rate_trend_item(90, 100, 80, 100);
        assert_eq!(item.rate, Some(0.9));
        assert_eq!(item.previous_rate, Some(0.8));
        assert!(matches!(item.trend, TrendBadge::Up));
        match item.change_pct {
            Some(p) => assert!((p - 12.5).abs() < 0.001),
            None => panic!("expected Some(12.5)"),
        }
    }
}
