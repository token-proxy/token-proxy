//! 审计日志仓储接口 — domain/log/
//!
//! 定义 `AuditLogRepository` trait，提供审计日志的持久化契约。

use async_trait::async_trait;

use crate::domain::log::audit_log::AuditLog;
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;

/// 审计日志仓储接口
#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    /// 保存审计日志
    async fn save(&self, log: &AuditLog) -> Result<(), AppError>;

    /// 分页查询审计日志（按时间倒序）
    async fn find_all_paginated(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResult<AuditLog>, AppError>;
}
