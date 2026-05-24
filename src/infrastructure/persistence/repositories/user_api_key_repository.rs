use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::IntoActiveModel;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::domain::entities::user_api_key::UserApiKey;
use crate::domain::repositories::user_api_key_repository::UserApiKeyRepository;
use crate::infrastructure::persistence::entities::user_api_key::{ActiveModel, Column, Entity};
use crate::shared::error::AppError;
use uuid::Uuid;

pub struct SeaOrmUserApiKeyRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmUserApiKeyRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        SeaOrmUserApiKeyRepository { db }
    }
}

#[async_trait]
impl UserApiKeyRepository for SeaOrmUserApiKeyRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserApiKey>, AppError> {
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

    async fn find_by_key_hash(&self, key_hash: &str) -> Result<Option<UserApiKey>, AppError> {
        let db = &*self.db;
        let model = Entity::find()
            .filter(Column::KeyHash.eq(key_hash))
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match model {
            Some(m) => Ok(Some(m.try_into()?)),
            None => Ok(None),
        }
    }

    async fn find_all_by_user(&self, user_id: Uuid) -> Result<Vec<UserApiKey>, AppError> {
        let db = &*self.db;
        let models = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .all(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<UserApiKey>, AppError>>()
    }

    async fn save(&self, key: &UserApiKey) -> Result<UserApiKey, AppError> {
        let db = &*self.db;
        let exists = Entity::find_by_id(key.id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .is_some();

        let active_model: ActiveModel = key.clone().into();

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

        Entity::find_by_id(key.id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .map(|m| m.try_into())
            .ok_or_else(|| AppError::Internal("保存后无法查询到 UserApiKey".to_string()))?
    }

    async fn revoke(&self, id: Uuid) -> Result<(), AppError> {
        let db = &*self.db;

        if let Some(model) = Entity::find_by_id(id)
            .filter(Column::Status.ne("disabled"))
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
        {
            let mut active: ActiveModel = model.into_active_model();
            active.status = sea_orm::Set("disabled".to_string());
            active
                .update(db)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        Ok(())
    }

    async fn update_last_used(&self, id: Uuid) -> Result<(), AppError> {
        let db = &*self.db;
        let now = Utc::now();

        let mut active: ActiveModel = Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("UserApiKey 未找到".to_string()))?
            .into_active_model();

        active.last_used_at = sea_orm::Set(Some(
            now.with_timezone(&chrono::FixedOffset::east_opt(0).expect("UTC offset")),
        ));

        active
            .update(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}
