//! Dashboard 聚合查询的领域读模型。
//!
//! 本模块定义 Dashboard 数据视图所需的中间数据结构（窗口、聚合行）。
//! 这些类型不属于业务实体，是 Repository 层为应用层 Service 提供的查询结果。
//!
//! ## 设计取舍
//!
//! Dashboard 是只读视图，需要在 SQL 层做 LEFT JOIN 容忍删除（users / accounts / providers）。
//! 因此 `username` / `account_name` 等字段都是 `Option<String>`，None 表示关联实体已删除。

use chrono::{DateTime, Utc};
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

/// KPI 聚合结果（单次 SQL 返回的 5 个标量）
#[derive(Debug, Clone)]
pub struct KpiAggregate {
    /// 总请求数
    pub request_count: i64,
    /// 总 token 数（来自 log_token_usage）
    pub total_tokens: i64,
    /// 去重活跃成员数（DISTINCT user_id）
    pub active_user_count: i64,
    /// 缓存命中 token 数（cache_read_input_tokens 之和）
    pub cache_read_tokens: i64,
    /// 缓存命中率分母（input_tokens + cache_read_input_tokens 之和）
    pub input_plus_cache_read_tokens: i64,
}

/// Sparkline 时间序列桶
///
/// 三条序列（请求 / Token / 成员数）共享同一桶时间，便于前端共用 X 轴。
#[derive(Debug, Clone)]
pub struct SparklineBucket {
    /// 桶起始时间（已按 date_trunc 截断到 hour 或 day）
    pub bucket_start: DateTime<Utc>,
    /// 该桶内请求数
    pub request_count: i64,
    /// 该桶内 token 总和
    pub total_tokens: i64,
    /// 该桶内活跃成员数（去重）
    pub active_user_count: i64,
}

/// 成员排行行
#[derive(Debug, Clone)]
pub struct TopUserRow {
    /// 成员 UUID
    pub user_id: Uuid,
    /// 用户名；None = users 表已无该用户（已被删除）
    pub username: Option<String>,
    /// 显示名
    pub display_name: Option<String>,
    /// 窗口内请求数
    pub request_count: i64,
    /// 窗口内 token 总消耗
    pub total_tokens: i64,
}

/// 账号排行行
#[derive(Debug, Clone)]
pub struct TopAccountRow {
    /// 账号 UUID
    pub account_id: Uuid,
    /// 账号名；None = accounts 表已无该账号（已被删除）
    pub account_name: Option<String>,
    /// 所属服务商 UUID
    pub provider_id: Option<Uuid>,
    /// 服务商名
    pub provider_name: Option<String>,
    /// 当前禁用原因（字符串化，None = 可用）
    pub disabled_reason: Option<String>,
    /// 输入 token 数
    pub input_tokens: i64,
    /// 输出 token 数
    pub output_tokens: i64,
    /// 缓存读取 token 数
    pub cache_read_tokens: i64,
    /// 缓存写入 token 数
    pub cache_creation_tokens: i64,
    /// 总 token 数（用于排序）
    pub total_tokens: i64,
}
