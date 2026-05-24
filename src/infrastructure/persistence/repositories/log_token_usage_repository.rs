use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::domain::entities::log_entry::LogTokenUsage;
use crate::domain::repositories::log_token_usage_repository::LogTokenUsageRepository;
use crate::infrastructure::persistence::entities::log_token_usage::{ActiveModel, Column, Entity};
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
        Entity::insert(ActiveModel::from(usage.clone()))
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn find_by_log_id(&self, log_id: Uuid) -> Result<Option<LogTokenUsage>, AppError> {
        let model = Entity::find()
            .filter(Column::LogId.eq(log_id))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        model.map(LogTokenUsage::try_from).transpose()
    }

    async fn find_by_session_id(&self, session_id: &str) -> Result<Vec<LogTokenUsage>, AppError> {
        let models = Entity::find()
            .filter(Column::SessionId.eq(session_id))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(LogTokenUsage::try_from)
            .collect::<Result<Vec<_>, _>>()
    }
}
