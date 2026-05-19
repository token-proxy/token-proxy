use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, QueryOrder, Set};

use crate::domain::entities::audit_log::AuditLog;
use crate::domain::repositories::audit_log_repository::AuditLogRepository;
use crate::infrastructure::persistence::entities::audit_log::{ActiveModel, Entity};
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;

/// SeaORM 实现的审计日志仓储
pub struct SeaOrmAuditLogRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmAuditLogRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        SeaOrmAuditLogRepository { db }
    }
}

#[async_trait]
impl AuditLogRepository for SeaOrmAuditLogRepository {
    async fn save(&self, log: &AuditLog) -> Result<(), AppError> {
        let db = &*self.db;

        use chrono::FixedOffset;
        let offset = FixedOffset::east_opt(0).expect("UTC offset");

        let active_model = ActiveModel {
            id: Set(log.id),
            user_id: Set(log.user_id),
            action: Set(log.action.clone()),
            entity_type: Set(log.entity_type.clone()),
            entity_id: Set(log.entity_id),
            details: Set(log.details.clone()),
            timestamp: Set(log.timestamp.with_timezone(&offset)),
        };

        Entity::insert(active_model)
            .exec(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn find_all_paginated(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResult<AuditLog>, AppError> {
        let db = &*self.db;

        let paginator = Entity::find()
            .order_by_desc(super::super::entities::audit_log::Column::Timestamp)
            .paginate(db, page_size);

        let items = paginator
            .fetch_page(page.max(1) - 1)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let total = paginator
            .num_items()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let domain_items = items
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<AuditLog>, AppError>>()?;

        Ok(PaginatedResult {
            items: domain_items,
            total,
            page,
            page_size,
        })
    }
}
