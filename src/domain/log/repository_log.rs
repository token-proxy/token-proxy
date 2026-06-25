//! 日志仓储接口 — domain/log/
//!
//! 定义 `LogRepository` trait 及其关联的查询/摘要 DTO，
//! 提供日志元数据、内容、词元用量的持久化契约。

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::log::{
    DashboardWindow, HeatmapCell, KpiAggregate, LogContent, LogMetadata, LogTokenUsage,
    QualityMetrics, SparklineBucket, TopAccessPointRow, TopModelRow,
};
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;
use async_trait::async_trait;

/// 日志查询过滤参数
#[derive(Debug, Clone, Default)]
pub struct LogQuery {
    pub session_id: Option<String>,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub status_code: Option<i16>,
    pub provider_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    /// 按是否中断过滤
    pub is_interrupted: Option<bool>,
}

/// 日志条目带词元用量摘要
#[derive(Debug, Clone)]
pub struct LogMetadataWithTokenSummary {
    pub entry: LogMetadata,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub cache_creation_input_tokens: Option<i32>,
    pub cache_read_input_tokens: Option<i32>,
    pub thinking_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
}

/// 会话摘要数据（统计请求数、各类型词元总量）
#[derive(Debug, Clone)]
pub struct SessionSummaryData {
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    pub start_time: DateTime<Utc>,
    pub request_count: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_creation_input_tokens: i64,
    pub total_cache_read_input_tokens: i64,
    pub total_thinking_tokens: i64,
    pub total_tokens: i64,
}

/// 会话查询过滤参数
#[derive(Debug, Clone, Default)]
pub struct SessionQuery {
    pub session_id: Option<String>,
    pub user_id: Option<Uuid>,
    pub access_point_id: Option<Uuid>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub status_code: Option<i16>,
}

/// 日志仓储接口
#[async_trait]
pub trait LogRepository: Send + Sync {
    /// 根据 ID 查找日志条目
    async fn find_by_id(&self, id: Uuid) -> Result<Option<LogMetadata>, AppError>;

    /// 根据会话 ID 查找日志条目
    async fn find_by_session_id(&self, session_id: &str) -> Result<Vec<LogMetadata>, AppError>;

    /// 分页查询日志条目（支持过滤）
    async fn find_all_paginated(
        &self,
        page: u64,
        page_size: u64,
        filter: &LogQuery,
    ) -> Result<PaginatedResult<LogMetadata>, AppError>;

    /// 保存日志条目
    async fn save(&self, entry: &LogMetadata) -> Result<LogMetadata, AppError>;

    /// 保存日志内容
    async fn save_content(&self, content: &LogContent) -> Result<(), AppError>;

    /// 根据日志 ID 查找日志内容
    async fn find_content_by_log_id(&self, log_id: Uuid) -> Result<Option<LogContent>, AppError>;

    /// 删除指定 ID 的日志条目及其内容
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;

    // ─── 联表查询 ───

    /// 分页查询日志条目（含词元用量摘要），使用 LEFT JOIN log_token_usage 联表查询
    async fn find_all_paginated_with_token_summary(
        &self,
        page: u64,
        page_size: u64,
        filter: &LogQuery,
    ) -> Result<PaginatedResult<LogMetadataWithTokenSummary>, AppError>;

    /// 分页查询会话摘要列表，使用聚合查询统计各会话的词元用量
    async fn find_sessions_paginated(
        &self,
        page: u64,
        page_size: u64,
        filter: &SessionQuery,
    ) -> Result<PaginatedResult<SessionSummaryData>, AppError>;

    /// 查询日志完整详情（联表查询 log_metadata + log_contents + log_token_usage）
    async fn find_log_detail_full(
        &self,
        id: Uuid,
    ) -> Result<Option<(LogMetadata, LogContent, Option<LogTokenUsage>)>, AppError>;

    // ─── Dashboard 聚合查询 ───

    /// 聚合 KPI 标量值（单次 SQL，5 个聚合列）
    ///
    /// 用于个人 Dashboard 顶部 KPI 卡。所有数据按 `user_id` 过滤，覆盖当前登录用户视角。
    /// 缓存命中率 = `cache_read_tokens / total_input_side_tokens`，分母为 0 时由调用方判定为 None。
    async fn aggregate_kpi(
        &self,
        user_id: Uuid,
        window: &DashboardWindow,
    ) -> Result<KpiAggregate, AppError>;

    /// 聚合 sparkline 时间序列（按 hour 或 day 分桶）
    ///
    /// 限定 `user_id` 范围。自动用 `generate_series` 补齐空桶，确保返回 `bucket_count` 个桶。
    /// `bucket_count = 24` 时按小时分桶（用于"今日"），否则按天分桶。
    async fn aggregate_sparkline(
        &self,
        user_id: Uuid,
        window: &DashboardWindow,
        bucket_count: u32,
    ) -> Result<Vec<SparklineBucket>, AppError>;

    /// 用户日级 365 天词元热力图（独立于 DashboardWindow）
    ///
    /// 按浏览器时区 `timezone`（已通过 application 层 chrono-tz 白名单校验）做 `AT TIME ZONE` 日级分桶；
    /// SQL 用 `generate_series` 补齐 365 天空桶，确保返回 365 行。
    async fn user_daily_token_heatmap(
        &self,
        user_id: Uuid,
        end: DateTime<Utc>,
        timezone: &str,
    ) -> Result<Vec<HeatmapCell>, AppError>;

    /// 用户视角模型 Top N（按 model_mapped 分组，request_count 降序）
    async fn top_models_for_user(
        &self,
        user_id: Uuid,
        window: &DashboardWindow,
        limit: u32,
    ) -> Result<Vec<TopModelRow>, AppError>;

    /// 用户视角接入点 Top N（LEFT JOIN access_points 容忍删除）
    async fn top_access_points_for_user(
        &self,
        user_id: Uuid,
        window: &DashboardWindow,
        limit: u32,
    ) -> Result<Vec<TopAccessPointRow>, AppError>;

    /// 用户视角调用质量指标（状态码分布 + 中断 + 平均/P95 耗时）
    async fn quality_metrics_for_user(
        &self,
        user_id: Uuid,
        window: &DashboardWindow,
    ) -> Result<QualityMetrics, AppError>;
}
