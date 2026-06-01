use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter};

use crate::domain::entities::access_point::AccessPoint;
use crate::domain::entities::account::Account;
use crate::domain::entities::provider::Provider;
use crate::domain::repositories::access_point_repository::AccessPointRepository;
use crate::domain::value_objects::status::Status;
use crate::domain::entities::access_point::{ActiveModel, Column, Entity};
use crate::domain::entities::account;
use crate::domain::entities::provider;
use crate::shared::error::AppError;
use uuid::Uuid;

pub struct SeaOrmAccessPointRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmAccessPointRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        SeaOrmAccessPointRepository { db }
    }
}

#[async_trait]
impl AccessPointRepository for SeaOrmAccessPointRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AccessPoint>, AppError> {
        Ok(Entity::find_by_id(id).one(&*self.db).await?)
    }

    async fn find_by_short_code(&self, short_code: &str) -> Result<Option<AccessPoint>, AppError> {
        Ok(Entity::find()
            .filter(Column::ShortCode.eq(short_code))
            .one(&*self.db)
            .await?)
    }

    async fn find_with_relations(
        &self,
        short_code: &str,
    ) -> Result<Option<(AccessPoint, Provider, Account)>, AppError> {
        let db = &*self.db;
        let ap_model = Entity::find()
            .filter(Column::ShortCode.eq(short_code))
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("接入点 '{}' 未找到", short_code)))?;

        let provider = ap_model
            .find_related(provider::Entity)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("接入点关联的提供商未找到".to_string()))?;

        let account = ap_model
            .find_related(account::Entity)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("接入点关联的账号未找到".to_string()))?;

        Ok(Some((ap_model, provider, account)))
    }

    async fn find_enabled(&self) -> Result<Vec<AccessPoint>, AppError> {
        Ok(Entity::find()
            .filter(Column::Status.eq(Status::Enabled))
            .all(&*self.db)
            .await?)
    }

    async fn find_by_provider_id(&self, provider_id: Uuid) -> Result<Vec<AccessPoint>, AppError> {
        Ok(Entity::find()
            .filter(Column::ProviderId.eq(provider_id))
            .all(&*self.db)
            .await?)
    }

    async fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<AccessPoint>, AppError> {
        Ok(Entity::find()
            .filter(Column::AccountId.eq(account_id))
            .all(&*self.db)
            .await?)
    }

    async fn find_all(&self) -> Result<Vec<AccessPoint>, AppError> {
        Ok(Entity::find().all(&*self.db).await?)
    }

    async fn save(&self, access_point: &AccessPoint) -> Result<AccessPoint, AppError> {
        let db = &*self.db;
        let exists = Entity::find_by_id(access_point.id).one(db).await?.is_some();

        let active_model: ActiveModel = access_point.clone().into();

        if exists {
            Entity::update(active_model).exec(db).await?;
        } else {
            Entity::insert(active_model).exec(db).await?;
        }

        Entity::find_by_id(access_point.id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Internal("保存后无法查询到 AccessPoint".to_string()))
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        Entity::delete_by_id(id).exec(&*self.db).await?;
        Ok(())
    }
}
