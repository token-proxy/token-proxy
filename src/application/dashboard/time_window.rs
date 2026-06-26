//! 时间窗口解析：将 `TimeRangeParams` 解析为当窗 + 上一窗 + 桶配置。
//!
//! 统一窗口计算，取消预设分支。桶粒度按跨度推导：
//! - 跨度 <= 1 天 → Hour（24 桶），跨度 > 1 天 → Day（N 桶）。

use chrono::Duration;

use crate::application::dashboard::dto::TimeRangeParams;
use crate::domain::log::dashboard_query::DashboardWindow;
use crate::shared::error::AppError;

/// 时间桶粒度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BucketGranularity {
    /// 按小时分桶（用于"今日"视图）
    Hour,
    /// 按天分桶（用于 7 天 / 30 天 / 自定义视图）
    Day,
}

/// 解析后的窗口配置
#[derive(Debug, Clone)]
pub struct ResolvedRanges {
    /// 当前查询窗口
    pub current: DashboardWindow,
    /// 上一等长窗口（用于对比）
    pub previous: DashboardWindow,
    /// 桶数（24 / 7 / 30 / N）
    pub bucket_count: u32,
    /// 桶粒度
    pub granularity: BucketGranularity,
}

/// 将 `TimeRangeParams` 解析为带对比窗口的 `ResolvedRanges`
///
/// 统一计算：当前窗 `[start, end)`，上一窗等长前移。
/// 桶粒度按跨度推导：跨度 <= 1 天 → Hour（24 桶），跨度 > 1 天 → Day（N 桶）。
pub fn resolve_windows(params: &TimeRangeParams) -> Result<ResolvedRanges, AppError> {
    if params.end <= params.start {
        return Err(AppError::Validation("end 必须晚于 start".to_string()));
    }
    let duration = params.end - params.start;
    // 不足 1 天按 1 天处理，确保至少有一个桶
    let days = duration.num_days().max(1) as u32;

    let (granularity, bucket_count) = if duration <= Duration::days(1) {
        (BucketGranularity::Hour, 24)
    } else {
        (BucketGranularity::Day, days)
    };

    let current = DashboardWindow {
        start: params.start,
        end: params.end,
    };
    let previous = DashboardWindow {
        start: params.start - duration,
        end: params.start,
    };

    Ok(ResolvedRanges {
        current,
        previous,
        bucket_count,
        granularity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn resolve_windows_rejects_inverted_range() {
        let now = Utc::now();
        let params = TimeRangeParams {
            start: now,
            end: now - Duration::days(1),
            tz: "UTC".to_string(),
        };
        assert!(matches!(
            resolve_windows(&params),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn resolve_windows_uses_hourly_for_short_spans() {
        let now = Utc::now();
        let params = TimeRangeParams {
            start: now - Duration::hours(12),
            end: now,
            tz: "UTC".to_string(),
        };
        let r = resolve_windows(&params).unwrap();
        assert_eq!(r.bucket_count, 24);
        assert_eq!(r.granularity, BucketGranularity::Hour);
    }

    #[test]
    fn resolve_windows_uses_daily_for_long_spans() {
        let now = Utc::now();
        let params = TimeRangeParams {
            start: now - Duration::days(30),
            end: now,
            tz: "UTC".to_string(),
        };
        let r = resolve_windows(&params).unwrap();
        assert_eq!(r.bucket_count, 30);
        assert_eq!(r.granularity, BucketGranularity::Day);
    }
}
