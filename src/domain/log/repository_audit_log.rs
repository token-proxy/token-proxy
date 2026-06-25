//! 审计日志仓储接口 — domain/log/
//!
//! 定义 `AuditLogRepository` trait 和审计日志查询所需的读模型（`AuditLogWithUsername`）
//! 与筛选条件（`AuditLogQuery`）。

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::audit_action::AuditAction;
use super::audit_entity_type::AuditEntityType;
use crate::domain::log::audit_log::AuditLog;
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;

/// 审计日志查询过滤参数
#[derive(Debug, Clone, Default)]
pub struct AuditLogQuery {
    /// 操作类型筛选（多选，为空表示不过滤）
    pub actions: Option<Vec<AuditAction>>,
    /// 实体类型筛选（多选，为空表示不过滤）
    pub entity_types: Option<Vec<AuditEntityType>>,
    /// 操作者 ID 精确匹配
    pub operator_id: Option<Uuid>,
    /// 操作者类型筛选（"user" 或 "system"）
    pub operator_type: Option<String>,
    /// 时间范围起始（含）
    pub start_time: Option<DateTime<Utc>>,
    /// 时间范围结束（含）
    pub end_time: Option<DateTime<Utc>>,
}

/// 审计日志行（含用户名，通过 LEFT JOIN users 获取）
#[derive(Debug, Clone)]
pub struct AuditLogWithUsername {
    pub log: AuditLog,
    /// 操作者用户名（系统操作时为 None，用户操作时为 users.display_name）
    pub username: Option<String>,
}

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

    /// 分页查询审计日志（支持筛选），LEFT JOIN users 获取操作者用户名
    async fn find_all_paginated_with_username(
        &self,
        page: u64,
        page_size: u64,
        query: &AuditLogQuery,
    ) -> Result<PaginatedResult<AuditLogWithUsername>, AppError>;
}
