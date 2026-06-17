use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::log::token_usage::{ActiveModel, Column, Entity};
use crate::domain::log::LogTokenUsage;
use crate::domain::log::LogTokenUsageRepository;
use crate::shared::error::AppError;

pub struct SeaOrmLogTokenUsageRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmLogTokenUsageRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl LogTokenUsageRepository for SeaOrmLogTokenUsageRepository {
    async fn save(&self, usage: &LogTokenUsage) -> Result<(), AppError> {
        let active_model: ActiveModel = usage.clone().into();
        Entity::insert(active_model).exec(&*self.db).await?;
        Ok(())
    }

    async fn find_by_log_id(&self, log_id: Uuid) -> Result<Option<LogTokenUsage>, AppError> {
        Ok(Entity::find()
            .filter(Column::LogId.eq(log_id))
            .one(&*self.db)
            .await?)
    }

    async fn find_by_session_id(&self, session_id: &str) -> Result<Vec<LogTokenUsage>, AppError> {
        Ok(Entity::find()
            .filter(Column::SessionId.eq(session_id))
            .all(&*self.db)
            .await?)
    }
}
