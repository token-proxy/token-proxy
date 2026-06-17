use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};

use crate::domain::provider::repository::ProviderRepository;
use crate::domain::provider::Provider;
use crate::domain::provider::{ProviderActiveModel, ProviderColumn, ProviderEntity};
use crate::domain::shared::Status;
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
        Ok(ProviderEntity::find_by_id(id).one(&*self.db).await?)
    }

    async fn find_enabled(&self) -> Result<Vec<Provider>, AppError> {
        Ok(ProviderEntity::find()
            .filter(ProviderColumn::Status.eq(Status::Enabled))
            .order_by_asc(ProviderColumn::Name)
            .all(&*self.db)
            .await?)
    }

    async fn find_all(&self) -> Result<Vec<Provider>, AppError> {
        Ok(ProviderEntity::find()
            .order_by_asc(ProviderColumn::Name)
            .all(&*self.db)
            .await?)
    }

    async fn save(&self, provider: &Provider) -> Result<Provider, AppError> {
        let db = &*self.db;
        let exists = ProviderEntity::find_by_id(provider.id)
            .one(db)
            .await?
            .is_some();

        let active_model: ProviderActiveModel = provider.clone().into();

        if exists {
            let active_model = active_model.reset_all();
            ProviderEntity::update(active_model).exec(db).await?;
        } else {
            ProviderEntity::insert(active_model).exec(db).await?;
        }

        ProviderEntity::find_by_id(provider.id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Internal("保存后无法查询到 Provider".to_string()))
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        ProviderEntity::delete_by_id(id).exec(&*self.db).await?;
        Ok(())
    }
}
