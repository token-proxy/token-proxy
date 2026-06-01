use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::domain::entities::provider::Provider;
use crate::domain::repositories::provider_repository::ProviderRepository;
use crate::domain::entities::provider::{ActiveModel, Column, Entity};
use crate::domain::value_objects::status::Status;
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
        Ok(Entity::find_by_id(id).one(&*self.db).await?)
    }

    async fn find_enabled(&self) -> Result<Vec<Provider>, AppError> {
        Ok(Entity::find()
            .filter(Column::Status.eq(Status::Enabled))
            .order_by_asc(Column::Name)
            .all(&*self.db)
            .await?)
    }

    async fn find_all(&self) -> Result<Vec<Provider>, AppError> {
        Ok(Entity::find()
            .order_by_asc(Column::Name)
            .all(&*self.db)
            .await?)
    }

    async fn save(&self, provider: &Provider) -> Result<Provider, AppError> {
        let db = &*self.db;
        let exists = Entity::find_by_id(provider.id).one(db).await?.is_some();

        let active_model: ActiveModel = provider.clone().into();

        if exists {
            Entity::update(active_model).exec(db).await?;
        } else {
            Entity::insert(active_model).exec(db).await?;
        }

        Entity::find_by_id(provider.id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Internal("保存后无法查询到 Provider".to_string()))
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        Entity::delete_by_id(id).exec(&*self.db).await?;
        Ok(())
    }
}
