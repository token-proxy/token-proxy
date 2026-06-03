use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, QueryOrder, Set};

use crate::domain::log::AuditLog;
use crate::domain::log::AuditLogRepository;
use crate::domain::log::audit_log::{ActiveModel, Column, Entity};
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;

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
        let active_model = ActiveModel {
            id: Set(log.id),
            user_id: Set(log.user_id),
            action: Set(log.action.clone()),
            entity_type: Set(log.entity_type.clone()),
            entity_id: Set(log.entity_id),
            details: Set(log.details.clone()),
            timestamp: Set(log.timestamp),
        };

        Entity::insert(active_model).exec(&*self.db).await?;
        Ok(())
    }

    async fn find_all_paginated(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResult<AuditLog>, AppError> {
        let db = &*self.db;
        let paginator = Entity::find()
            .order_by_desc(Column::Timestamp)
            .paginate(db, page_size);

        let items = paginator.fetch_page(page.max(1) - 1).await?;
        let total = paginator.num_items().await?;

        Ok(PaginatedResult { items, total, page, page_size })
    }
}
