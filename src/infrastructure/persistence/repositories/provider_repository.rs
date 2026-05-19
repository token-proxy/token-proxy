use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::domain::entities::provider::Provider;
use crate::domain::repositories::provider_repository::ProviderRepository;
use crate::infrastructure::persistence::entities::provider::{ActiveModel, Column, Entity};
use crate::shared::error::AppError;
use uuid::Uuid;

pub struct SeaOrmProviderRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmProviderRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        SeaOrmProviderRepository { db }
    }
}

#[async_trait]
impl ProviderRepository for SeaOrmProviderRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Provider>, AppError> {
        let db = &*self.db;
        let model = Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match model {
            Some(m) => Ok(Some(m.try_into()?)),
            None => Ok(None),
        }
    }

    async fn find_enabled(&self) -> Result<Vec<Provider>, AppError> {
        let db = &*self.db;
        let models = Entity::find()
            .filter(Column::Status.eq("enabled"))
            .order_by_asc(super::super::entities::provider::Column::Name)
            .all(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<Provider>, AppError>>()
    }

    async fn find_all(&self) -> Result<Vec<Provider>, AppError> {
        let db = &*self.db;
        let models = Entity::find()
            .order_by_asc(super::super::entities::provider::Column::Name)
            .all(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<Provider>, AppError>>()
    }

    async fn save(&self, provider: &Provider) -> Result<Provider, AppError> {
        let db = &*self.db;
        let exists = Entity::find_by_id(provider.id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .is_some();

        let active_model: ActiveModel = provider.clone().into();

        if exists {
            Entity::update(active_model)
                .exec(db)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        } else {
            Entity::insert(active_model)
                .exec(db)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        // 重新查询以返回完整的最新数据
        Entity::find_by_id(provider.id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .map(|m| m.try_into())
            .ok_or_else(|| AppError::Internal("保存后无法查询到 Provider".to_string()))?
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let db = &*self.db;
        Entity::delete_by_id(id)
            .exec(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}