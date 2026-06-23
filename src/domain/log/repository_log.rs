//! 日志仓储接口 — domain/log/
//!
//! 定义 `LogRepository` trait 及其关联的查询/摘要 DTO，
//! 提供日志元数据、内容、token 用量的持久化契约。

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::log::{
    DashboardWindow, KpiAggregate, LogContent, LogMetadata, LogTokenUsage, SparklineBucket,
    TopAccountRow, TopUserRow,
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

/// 日志条目带 token 用量摘要
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

/// 会话摘要数据（统计请求数、各类型 token 总量）
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

    /// 分页查询日志条目（含 token 用量摘要），使用 LEFT JOIN log_token_usage 联表查询
    async fn find_all_paginated_with_token_summary(
        &self,
        page: u64,
        page_size: u64,
        filter: &LogQuery,
    ) -> Result<PaginatedResult<LogMetadataWithTokenSummary>, AppError>;

    /// 分页查询会话摘要列表，使用聚合查询统计各会话的 token 用量
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
    /// 用于 Dashboard 顶部 4 张 KPI 卡。缓存命中率 = `cache_read_tokens / input_plus_cache_read_tokens`，
    /// 分母为 0 时由调用方判定为 None。
    async fn aggregate_kpi(&self, window: &DashboardWindow) -> Result<KpiAggregate, AppError>;

    /// 聚合 sparkline 时间序列（按 hour 或 day 分桶）
    ///
    /// 自动用 `generate_series` 补齐空桶，确保返回 `bucket_count` 个桶。
    /// `bucket_count = 24` 时按小时分桶（用于"今日"），否则按天分桶。
    async fn aggregate_sparkline(
        &self,
        window: &DashboardWindow,
        bucket_count: u32,
    ) -> Result<Vec<SparklineBucket>, AppError>;

    /// 成员请求量排行 Top N
    ///
    /// LEFT JOIN users 表容忍删除：被删除成员的 `username` / `display_name` 返回 None。
    async fn top_users(
        &self,
        window: &DashboardWindow,
        limit: u32,
    ) -> Result<Vec<TopUserRow>, AppError>;

    /// 账号 Token 消耗排行 Top N
    ///
    /// LEFT JOIN accounts + providers 表容忍删除：被删除账号 / 服务商的对应字段返回 None。
    async fn top_accounts(
        &self,
        window: &DashboardWindow,
        limit: u32,
    ) -> Result<Vec<TopAccountRow>, AppError>;
}
