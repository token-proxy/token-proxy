use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};

use crate::domain::user::refresh_token::{
    ActiveModel as RefreshTokenActiveModel, Column as RefreshTokenColumn,
    Entity as RefreshTokenEntity,
};
use crate::domain::user::RefreshToken;
use crate::domain::user::RefreshTokenRepository;
use crate::shared::error::AppError;
use uuid::Uuid;

pub struct SeaOrmRefreshTokenRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmRefreshTokenRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        SeaOrmRefreshTokenRepository { db }
    }
}

#[async_trait]
impl RefreshTokenRepository for SeaOrmRefreshTokenRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<RefreshToken>, AppError> {
        Ok(RefreshTokenEntity::find_by_id(id).one(&*self.db).await?)
    }

    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<RefreshToken>, AppError> {
        Ok(RefreshTokenEntity::find()
            .filter(RefreshTokenColumn::TokenHash.eq(token_hash))
            .one(&*self.db)
            .await?)
    }

    async fn find_valid_by_user_id(&self, user_id: Uuid) -> Result<Vec<RefreshToken>, AppError> {
        Ok(RefreshTokenEntity::find()
            .filter(RefreshTokenColumn::UserId.eq(user_id))
            .filter(RefreshTokenColumn::Revoked.eq(false))
            .filter(RefreshTokenColumn::ExpiresAt.gt(Utc::now()))
            .order_by_desc(RefreshTokenColumn::CreatedAt)
            .all(&*self.db)
            .await?)
    }

    async fn save(&self, token: &RefreshToken) -> Result<RefreshToken, AppError> {
        let db = &*self.db;
        let exists = RefreshTokenEntity::find_by_id(token.id)
            .one(db)
            .await?
            .is_some();

        let active_model: RefreshTokenActiveModel = token.clone().into();

        if exists {
            let active_model = active_model.reset_all();
            RefreshTokenEntity::update(active_model).exec(db).await?;
        } else {
            RefreshTokenEntity::insert(active_model).exec(db).await?;
        }

        RefreshTokenEntity::find_by_id(token.id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Internal("保存后无法查询到 RefreshToken".to_string()))
    }

    async fn revoke(&self, id: Uuid) -> Result<(), AppError> {
        let db = &*self.db;
        if let Some(model) = RefreshTokenEntity::find_by_id(id)
            .filter(RefreshTokenColumn::Revoked.eq(false))
            .one(db)
            .await?
        {
            let mut active: RefreshTokenActiveModel = model.into();
            active.revoked = sea_orm::Set(true);
            active.update(db).await?;
        }
        Ok(())
    }

    async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<(), AppError> {
        let db = &*self.db;
        let models = RefreshTokenEntity::find()
            .filter(RefreshTokenColumn::UserId.eq(user_id))
            .filter(RefreshTokenColumn::Revoked.eq(false))
            .all(db)
            .await?;

        for model in models {
            let mut active: RefreshTokenActiveModel = model.into();
            active.revoked = sea_orm::Set(true);
            active.update(db).await?;
        }
        Ok(())
    }

    async fn delete_expired(&self) -> Result<u64, AppError> {
        Ok(RefreshTokenEntity::delete_many()
            .filter(RefreshTokenColumn::ExpiresAt.lt(Utc::now()))
            .exec(&*self.db)
            .await?
            .rows_affected)
    }
}
