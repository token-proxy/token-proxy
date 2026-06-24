//! Dashboard 应用层 Service：编排 KPI / 排行 / 时间序列查询。
//!
//! ## 职责
//!
//! - 解析 `TimeRangeQuery` 为当窗 + 上一窗（委托 `time_window::resolve_windows`）
//! - 调度 `LogRepository` 聚合方法
//! - 计算 KPI 趋势徽章（`compute_trend` 纯函数）
//! - 装配 DTO 返回给展示层
//!
//! ## 设计纪律
//!
//! 本 Service 仅依赖 `LogRepository`（事实表入口），所有跨表 LEFT JOIN
//! 在 Repository 层完成，避免 N+1 查询和跨聚合依赖。

use std::sync::Arc;

use crate::application::dashboard::dto::{
    CacheHitRate, KpiResponse, KpiTrendItem, SparklineBucketDto, SparklineSeries, TimeRangeQuery,
    TopAccountItem, TopAccountsResponse, TopUserItem, TopUsersResponse, TrendBadge,
};
use crate::application::dashboard::time_window::resolve_windows;
use crate::domain::log::dashboard_query::KpiAggregate;
use crate::domain::log::repository_log::LogRepository;
use crate::shared::error::AppError;

/// 排行 Top N 默认条数
const RANKING_LIMIT: u32 = 10;

/// 趋势"持平"判定阈值（绝对百分比 < FLAT_THRESHOLD 视为持平）
const FLAT_THRESHOLD: f64 = 0.5;

/// Dashboard 应用层 Service
pub struct DashboardService {
    log_repository: Arc<dyn LogRepository>,
}

impl DashboardService {
    /// 构造 DashboardService
    pub fn new(log_repository: Arc<dyn LogRepository>) -> Self {
        Self { log_repository }
    }

    /// 获取 4 张 KPI 卡 + 内嵌 sparkline 序列
    #[tracing::instrument(skip_all, fields(range = ?q.range))]
    pub async fn get_kpi(&self, q: TimeRangeQuery) -> Result<KpiResponse, AppError> {
        let ranges = resolve_windows(&q)?;

        // 1. 并行聚合：当窗 KPI、上一窗 KPI、当窗 sparkline
        let (current, previous, sparkline) = tokio::try_join!(
            self.log_repository.aggregate_kpi(&ranges.current),
            self.log_repository.aggregate_kpi(&ranges.previous),
            self.log_repository
                .aggregate_sparkline(&ranges.current, ranges.bucket_count),
        )?;

        // 2. 装配 4 张 KPI 卡
        let request_count = build_trend_item(current.request_count, previous.request_count);
        let total_tokens = build_trend_item(current.total_tokens, previous.total_tokens);
        let active_user_count =
            build_trend_item(current.active_user_count, previous.active_user_count);
        let cache_hit_rate = build_cache_hit_rate(&current, &previous);

        // 3. 转换 sparkline 桶为 DTO
        let buckets: Vec<SparklineBucketDto> = sparkline
            .into_iter()
            .map(|b| SparklineBucketDto {
                bucket_start: b.bucket_start,
                request_count: b.request_count,
                total_tokens: b.total_tokens,
                active_user_count: b.active_user_count,
            })
            .collect();

        Ok(KpiResponse {
            request_count,
            total_tokens,
            active_user_count,
            cache_hit_rate,
            sparkline: SparklineSeries { buckets },
        })
    }

    /// 获取成员请求量排行 Top N
    #[tracing::instrument(skip_all, fields(range = ?q.range))]
    pub async fn get_top_users(&self, q: TimeRangeQuery) -> Result<TopUsersResponse, AppError> {
        let ranges = resolve_windows(&q)?;
        let rows = self
            .log_repository
            .top_users(&ranges.current, RANKING_LIMIT)
            .await?;
        let items = rows
            .into_iter()
            .map(|r| TopUserItem {
                user_id: r.user_id,
                username: r.username,
                display_name: r.display_name,
                request_count: r.request_count,
                total_tokens: r.total_tokens,
            })
            .collect();
        Ok(TopUsersResponse { items })
    }

    /// 获取上游账号词元消耗排行 Top N
    #[tracing::instrument(skip_all, fields(range = ?q.range))]
    pub async fn get_top_accounts(
        &self,
        q: TimeRangeQuery,
    ) -> Result<TopAccountsResponse, AppError> {
        let ranges = resolve_windows(&q)?;
        let rows = self
            .log_repository
            .top_accounts(&ranges.current, RANKING_LIMIT)
            .await?;
        let items = rows
            .into_iter()
            .map(|r| TopAccountItem {
                account_id: r.account_id,
                account_name: r.account_name,
                provider_id: r.provider_id,
                provider_name: r.provider_name,
                disabled_reason: r.disabled_reason,
                input_tokens: r.input_tokens,
                output_tokens: r.output_tokens,
                cache_read_tokens: r.cache_read_tokens,
                cache_creation_tokens: r.cache_creation_tokens,
                total_tokens: r.total_tokens,
            })
            .collect();
        Ok(TopAccountsResponse { items })
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
    let rate = compute_rate(
        current.cache_read_tokens,
        current.input_plus_cache_read_tokens,
    );
    let previous_rate = compute_rate(
        previous.cache_read_tokens,
        previous.input_plus_cache_read_tokens,
    );
    let (trend, change_pct) = compute_rate_trend(rate, previous_rate);
    CacheHitRate {
        rate,
        previous_rate,
        change_pct,
        trend,
    }
}

/// 计算缓存命中率：`cache_read / (input + cache_read)`；分母为 0 返回 None
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
}
