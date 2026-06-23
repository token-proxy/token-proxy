//! 时间窗口解析：将 `TimeRangeQuery` 转换为 (当窗 + 上一窗 + 桶配置)。
//!
//! 这是纯逻辑模块，无 I/O 依赖，可在应用层独立单元测试。

use chrono::{DateTime, Duration, TimeZone, Utc};

use crate::application::dashboard::dto::{TimeRangePreset, TimeRangeQuery};
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

/// 将 `TimeRangeQuery` 解析为带对比窗口的 `ResolvedRanges`
///
/// 规则：
/// - `Today` —— 当前 = 今日 00:00 到 现在；上一窗 = 昨日同时段；24 小时桶
/// - `Last7` —— 当前 = 现在 - 7 天 到 现在；上一窗 = 上 7 天；7 日桶
/// - `Last30` —— 当前 = 现在 - 30 天 到 现在；上一窗 = 上 30 天；30 日桶
/// - `Custom` —— 用户提供 start / end；上一窗 = 等长前期；按天数分日桶
pub fn resolve_windows(q: &TimeRangeQuery) -> Result<ResolvedRanges, AppError> {
    let now = Utc::now();
    match q.range {
        TimeRangePreset::Today => {
            // 1. 计算今日 00:00:00 UTC 起点
            let today_start = now
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .and_then(|naive| Utc.from_local_datetime(&naive).single())
                .ok_or_else(|| AppError::Internal("无法构造今日起点时间戳".to_string()))?;
            // 2. 当前窗：今日 00:00 到 现在
            let current = DashboardWindow {
                start: today_start,
                end: now,
            };
            // 3. 上一窗：昨日同时段，整体平移 24 小时
            let previous = DashboardWindow {
                start: today_start - Duration::days(1),
                end: now - Duration::days(1),
            };
            Ok(ResolvedRanges {
                current,
                previous,
                bucket_count: 24,
                granularity: BucketGranularity::Hour,
            })
        }
        TimeRangePreset::Last7 => Ok(build_rolling_window(now, 7)),
        TimeRangePreset::Last30 => Ok(build_rolling_window(now, 30)),
        TimeRangePreset::Custom => {
            let start = q
                .start
                .ok_or_else(|| AppError::Validation("自定义时间范围必须提供 start".to_string()))?;
            let end = q
                .end
                .ok_or_else(|| AppError::Validation("自定义时间范围必须提供 end".to_string()))?;
            if end <= start {
                return Err(AppError::Validation("end 必须晚于 start".to_string()));
            }
            let duration = end - start;
            // 不足 1 天按 1 天处理，确保至少有一个桶
            let days = duration.num_days().max(1) as u32;
            Ok(ResolvedRanges {
                current: DashboardWindow { start, end },
                previous: DashboardWindow {
                    start: start - duration,
                    end: start,
                },
                bucket_count: days,
                granularity: BucketGranularity::Day,
            })
        }
    }
}

/// 构造滚动窗口（用于 Last7 / Last30）
fn build_rolling_window(now: DateTime<Utc>, days: u32) -> ResolvedRanges {
    let duration = Duration::days(days as i64);
    let current = DashboardWindow {
        start: now - duration,
        end: now,
    };
    let previous = DashboardWindow {
        start: now - duration * 2,
        end: now - duration,
    };
    ResolvedRanges {
        current,
        previous,
        bucket_count: days,
        granularity: BucketGranularity::Day,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_windows_last7_is_7_days_long() {
        let q = TimeRangeQuery {
            range: TimeRangePreset::Last7,
            start: None,
            end: None,
        };
        let r = resolve_windows(&q).unwrap();
        assert_eq!(r.bucket_count, 7);
        assert_eq!(r.granularity, BucketGranularity::Day);
        let span = r.current.end - r.current.start;
        assert_eq!(span.num_days(), 7);
    }

    #[test]
    fn resolve_windows_last30_previous_is_offset() {
        let q = TimeRangeQuery {
            range: TimeRangePreset::Last30,
            start: None,
            end: None,
        };
        let r = resolve_windows(&q).unwrap();
        // 上一窗的结束时刻应等于当前窗的起始时刻
        assert_eq!(r.current.start, r.previous.end);
        assert_eq!(r.bucket_count, 30);
    }

    #[test]
    fn resolve_windows_today_uses_hour_granularity() {
        let q = TimeRangeQuery {
            range: TimeRangePreset::Today,
            start: None,
            end: None,
        };
        let r = resolve_windows(&q).unwrap();
        assert_eq!(r.bucket_count, 24);
        assert_eq!(r.granularity, BucketGranularity::Hour);
    }

    #[test]
    fn resolve_windows_custom_requires_start_and_end() {
        let q = TimeRangeQuery {
            range: TimeRangePreset::Custom,
            start: None,
            end: None,
        };
        assert!(matches!(resolve_windows(&q), Err(AppError::Validation(_))));
    }

    #[test]
    fn resolve_windows_custom_rejects_inverted_range() {
        let now = Utc::now();
        let q = TimeRangeQuery {
            range: TimeRangePreset::Custom,
            start: Some(now),
            end: Some(now - Duration::days(1)),
        };
        assert!(matches!(resolve_windows(&q), Err(AppError::Validation(_))));
    }
}
