use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::domain::entities::log_entry::{LogContent, LogEntry};
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
}

#[async_trait]
pub trait LogRepository: Send + Sync {
    /// 根据 ID 查找日志条目
    async fn find_by_id(&self, id: Uuid) -> Result<Option<LogEntry>, AppError>;

    /// 根据会话 ID 查找日志条目
    async fn find_by_session_id(&self, session_id: &str) -> Result<Vec<LogEntry>, AppError>;

    /// 分页查询日志条目（支持过滤）
    async fn find_all_paginated(
        &self,
        page: u64,
        page_size: u64,
        filter: &LogQuery,
    ) -> Result<PaginatedResult<LogEntry>, AppError>;

    /// 保存日志条目
    async fn save(&self, entry: &LogEntry) -> Result<LogEntry, AppError>;

    /// 保存日志内容
    async fn save_content(&self, content: &LogContent) -> Result<(), AppError>;

    /// 根据日志 ID 查找日志内容
    async fn find_content_by_log_id(&self, log_id: Uuid) -> Result<Option<LogContent>, AppError>;

    /// 删除指定 ID 的日志条目及其内容
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;

    // ─── 统计方法 ───

    /// 统计日志总条数
    async fn count_total(&self) -> Result<u64, AppError>;

    /// 按日期范围统计请求量（返回每天的日期和请求数）
    async fn count_by_date_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<(NaiveDate, u64)>, AppError>;

    /// Top N 接入点排名
    async fn top_access_points(&self, limit: u64) -> Result<Vec<(Uuid, u64)>, AppError>;

    /// Top N 模型排名
    async fn top_models(&self, limit: u64) -> Result<Vec<(String, u64)>, AppError>;

    /// 统计有日志记录的活跃接入点数量
    async fn count_active_access_points(&self) -> Result<u64, AppError>;
}
