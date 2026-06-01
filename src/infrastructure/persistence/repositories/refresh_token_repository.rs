use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};

use crate::domain::entities::refresh_token::RefreshToken;
use crate::domain::repositories::refresh_token_repository::RefreshTokenRepository;
use crate::domain::entities::refresh_token::{ActiveModel, Column, Entity};
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
        Ok(Entity::find_by_id(id).one(&*self.db).await?)
    }

    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<RefreshToken>, AppError> {
        Ok(Entity::find()
            .filter(Column::TokenHash.eq(token_hash))
            .one(&*self.db)
            .await?)
    }

    async fn find_valid_by_user_id(&self, user_id: Uuid) -> Result<Vec<RefreshToken>, AppError> {
        Ok(Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::Revoked.eq(false))
            .filter(Column::ExpiresAt.gt(Utc::now()))
            .order_by_desc(Column::CreatedAt)
            .all(&*self.db)
            .await?)
    }

    async fn save(&self, token: &RefreshToken) -> Result<RefreshToken, AppError> {
        let db = &*self.db;
        let exists = Entity::find_by_id(token.id).one(db).await?.is_some();

        let active_model: ActiveModel = token.clone().into();

        if exists {
            Entity::update(active_model).exec(db).await?;
        } else {
            Entity::insert(active_model).exec(db).await?;
        }

        Entity::find_by_id(token.id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Internal("保存后无法查询到 RefreshToken".to_string()))
    }

    async fn revoke(&self, id: Uuid) -> Result<(), AppError> {
        let db = &*self.db;
        if let Some(model) = Entity::find_by_id(id)
            .filter(Column::Revoked.eq(false))
            .one(db)
            .await?
        {
            let mut active: ActiveModel = model.into();
            active.revoked = sea_orm::Set(true);
            active.update(db).await?;
        }
        Ok(())
    }

    async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<(), AppError> {
        let db = &*self.db;
        let models = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::Revoked.eq(false))
            .all(db)
            .await?;

        for model in models {
            let mut active: ActiveModel = model.into();
            active.revoked = sea_orm::Set(true);
            active.update(db).await?;
        }
        Ok(())
    }

    async fn delete_expired(&self) -> Result<u64, AppError> {
        Ok(Entity::delete_many()
            .filter(Column::ExpiresAt.lt(Utc::now()))
            .exec(&*self.db)
            .await?
            .rows_affected)
    }
}
