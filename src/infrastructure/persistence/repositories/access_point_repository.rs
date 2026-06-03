use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter};

use crate::domain::access_point::AccessPoint;
use crate::domain::access_point::repository::AccessPointRepository;
use crate::domain::shared::Status;
use crate::domain::access_point::{
    AccessPointActiveModel, AccessPointColumn, AccessPointEntity, AccessPointEx,
};
use crate::domain::provider::{AccountEntity, ProviderEntity};
use crate::shared::error::AppError;
use uuid::Uuid;

use sea_orm::entity::compound::HasOne;

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
        Ok(AccessPointEntity::find_by_id(id).one(&*self.db).await?)
    }

    async fn find_by_short_code(&self, short_code: &str) -> Result<Option<AccessPointEx>, AppError> {
        let ap = AccessPointEntity::find()
            .filter(AccessPointColumn::ShortCode.eq(short_code))
            .one(&*self.db)
            .await?;

        match ap {
            Some(ap_model) => {
                let provider = ap_model
                    .find_related(ProviderEntity)
                    .one(&*self.db)
                    .await?;
                let account = ap_model
                    .find_related(AccountEntity)
                    .one(&*self.db)
                    .await?;

                let mut model_ex: AccessPointEx = ap_model.into();

                if let Some(p) = provider {
                    model_ex.provider = HasOne::loaded(p);
                } else {
                    model_ex.provider = HasOne::NotFound;
                }

                if let Some(a) = account {
                    model_ex.account = HasOne::loaded(a);
                } else {
                    model_ex.account = HasOne::NotFound;
                }

                Ok(Some(model_ex))
            }
            None => Ok(None),
        }
    }

    async fn find_enabled(&self) -> Result<Vec<AccessPoint>, AppError> {
        Ok(AccessPointEntity::find()
            .filter(AccessPointColumn::Status.eq(Status::Enabled))
            .all(&*self.db)
            .await?)
    }

    async fn find_by_provider_id(&self, provider_id: Uuid) -> Result<Vec<AccessPoint>, AppError> {
        Ok(AccessPointEntity::find()
            .filter(AccessPointColumn::ProviderId.eq(provider_id))
            .all(&*self.db)
            .await?)
    }

    async fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<AccessPoint>, AppError> {
        Ok(AccessPointEntity::find()
            .filter(AccessPointColumn::AccountId.eq(account_id))
            .all(&*self.db)
            .await?)
    }

    async fn find_all(&self) -> Result<Vec<AccessPoint>, AppError> {
        Ok(AccessPointEntity::find().all(&*self.db).await?)
    }

    async fn save(&self, access_point: &AccessPoint) -> Result<AccessPoint, AppError> {
        let db = &*self.db;
        let exists = AccessPointEntity::find_by_id(access_point.id).one(db).await?.is_some();

        let active_model: AccessPointActiveModel = access_point.clone().into();

        if exists {
            AccessPointEntity::update(active_model).exec(db).await?;
        } else {
            AccessPointEntity::insert(active_model).exec(db).await?;
        }

        AccessPointEntity::find_by_id(access_point.id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Internal("保存后无法查询到 AccessPoint".to_string()))
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        AccessPointEntity::delete_by_id(id).exec(&*self.db).await?;
        Ok(())
    }
}
