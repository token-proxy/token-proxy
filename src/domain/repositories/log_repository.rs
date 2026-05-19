use crate::domain::entities::log_entry::{LogContent, LogEntry};
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait LogRepository: Send + Sync {
    /// 根据 ID 查找日志条目
    async fn find_by_id(&self, id: Uuid) -> Result<Option<LogEntry>, AppError>;

    /// 根据会话 ID 查找日志条目
    async fn find_by_session_id(&self, session_id: &str) -> Result<Vec<LogEntry>, AppError>;

    /// 分页查询所有日志条目
    async fn find_all_paginated(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResult<LogEntry>, AppError>;

    /// 保存日志条目
    async fn save(&self, entry: &LogEntry) -> Result<LogEntry, AppError>;

    /// 保存日志内容
    async fn save_content(&self, content: &LogContent) -> Result<(), AppError>;

    /// 根据日志 ID 查找日志内容
    async fn find_content_by_log_id(&self, log_id: Uuid) -> Result<Option<LogContent>, AppError>;

    /// 删除指定 ID 的日志条目及其内容
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
}