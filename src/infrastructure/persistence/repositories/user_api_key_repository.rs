use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::IntoActiveModel;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::domain::shared::Status;
use crate::domain::user::user_api_key::{
    ActiveModel as UserApiKeyActiveModel, Column as UserApiKeyColumn, Entity as UserApiKeyEntity,
};
use crate::domain::user::UserApiKey;
use crate::domain::user::UserApiKeyRepository;
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
        Ok(UserApiKeyEntity::find_by_id(id).one(&*self.db).await?)
    }

    async fn find_by_key_hash(&self, key_hash: &str) -> Result<Option<UserApiKey>, AppError> {
        Ok(UserApiKeyEntity::find()
            .filter(UserApiKeyColumn::KeyHash.eq(key_hash))
            .one(&*self.db)
            .await?)
    }

    async fn find_all_by_user(&self, user_id: Uuid) -> Result<Vec<UserApiKey>, AppError> {
        Ok(UserApiKeyEntity::find()
            .filter(UserApiKeyColumn::UserId.eq(user_id))
            .all(&*self.db)
            .await?)
    }

    async fn save(&self, key: &UserApiKey) -> Result<UserApiKey, AppError> {
        let db = &*self.db;
        let exists = UserApiKeyEntity::find_by_id(key.id)
            .one(db)
            .await?
            .is_some();

        let active_model: UserApiKeyActiveModel = key.clone().into_active_model();

        if exists {
            UserApiKeyEntity::update(active_model).exec(db).await?;
        } else {
            UserApiKeyEntity::insert(active_model).exec(db).await?;
        }

        UserApiKeyEntity::find_by_id(key.id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Internal("保存后无法查询到 UserApiKey".to_string()))
    }

    async fn revoke(&self, id: Uuid) -> Result<(), AppError> {
        let db = &*self.db;
        if let Some(model) = UserApiKeyEntity::find_by_id(id)
            .filter(UserApiKeyColumn::Status.ne(Status::Disabled))
            .one(db)
            .await?
        {
            let mut active: UserApiKeyActiveModel = model.into_active_model();
            active.status = sea_orm::Set(Status::Disabled);
            active.update(db).await?;
        }
        Ok(())
    }

    async fn update_last_used(&self, id: Uuid) -> Result<(), AppError> {
        let db = &*self.db;

        let mut active: UserApiKeyActiveModel = UserApiKeyEntity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("UserApiKey 未找到".to_string()))?
            .into_active_model();

        active.last_used_at = sea_orm::Set(Some(
            Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).expect("UTC offset")),
        ));

        active.update(db).await?;
        Ok(())
    }
}
