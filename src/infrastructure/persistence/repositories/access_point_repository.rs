use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::domain::entities::access_point::AccessPoint;
use crate::domain::repositories::access_point_repository::AccessPointRepository;
use crate::infrastructure::persistence::entities::access_point::{ActiveModel, Column, Entity};
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

    async fn find_by_short_code(&self, short_code: &str) -> Result<Option<AccessPoint>, AppError> {
        let db = &*self.db;
        let model = Entity::find()
            .filter(Column::ShortCode.eq(short_code))
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match model {
            Some(m) => Ok(Some(m.try_into()?)),
            None => Ok(None),
        }
    }

    async fn find_enabled(&self) -> Result<Vec<AccessPoint>, AppError> {
        let db = &*self.db;
        let models = Entity::find()
            .filter(Column::Status.eq("enabled"))
            .all(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<AccessPoint>, AppError>>()
    }

    async fn find_by_provider_id(&self, provider_id: Uuid) -> Result<Vec<AccessPoint>, AppError> {
        let db = &*self.db;
        let models = Entity::find()
            .filter(Column::ProviderId.eq(provider_id))
            .all(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<AccessPoint>, AppError>>()
    }

    async fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<AccessPoint>, AppError> {
        let db = &*self.db;
        let models = Entity::find()
            .filter(Column::AccountId.eq(account_id))
            .all(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<AccessPoint>, AppError>>()
    }

    async fn find_all(&self) -> Result<Vec<AccessPoint>, AppError> {
        let db = &*self.db;
        let models = Entity::find()
            .all(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<AccessPoint>, AppError>>()
    }

    async fn save(&self, access_point: &AccessPoint) -> Result<AccessPoint, AppError> {
        let db = &*self.db;
        let exists = Entity::find_by_id(access_point.id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .is_some();

        let active_model: ActiveModel = access_point.clone().into();

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

        Entity::find_by_id(access_point.id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .map(|m| m.try_into())
            .ok_or_else(|| AppError::Internal("保存后无法查询到 AccessPoint".to_string()))?
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