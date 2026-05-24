use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};

use crate::domain::entities::refresh_token::RefreshToken;
use crate::domain::repositories::refresh_token_repository::RefreshTokenRepository;
use crate::infrastructure::persistence::entities::refresh_token::{ActiveModel, Column, Entity};
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

    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<RefreshToken>, AppError> {
        let db = &*self.db;
        let model = Entity::find()
            .filter(Column::TokenHash.eq(token_hash))
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match model {
            Some(m) => Ok(Some(m.try_into()?)),
            None => Ok(None),
        }
    }

    async fn find_valid_by_user_id(&self, user_id: Uuid) -> Result<Vec<RefreshToken>, AppError> {
        let db = &*self.db;
        let now = Utc::now();

        let models = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::Revoked.eq(false))
            .filter(Column::ExpiresAt.gt(now))
            .order_by_desc(Column::CreatedAt)
            .all(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<RefreshToken>, AppError>>()
    }

    async fn save(&self, token: &RefreshToken) -> Result<RefreshToken, AppError> {
        let db = &*self.db;
        let exists = Entity::find_by_id(token.id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .is_some();

        let active_model: ActiveModel = token.clone().into();

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

        Entity::find_by_id(token.id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .map(|m| m.try_into())
            .ok_or_else(|| AppError::Internal("保存后无法查询到 RefreshToken".to_string()))?
    }

    /// 撤销指定 token（仅当尚未撤销时）
    async fn revoke(&self, id: Uuid) -> Result<(), AppError> {
        let db = &*self.db;

        match Entity::find_by_id(id)
            .filter(Column::Revoked.eq(false))
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
        {
            Some(model) => {
                let mut active: ActiveModel = model.into();
                active.revoked = sea_orm::Set(true);
                active
                    .update(db)
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;
            }
            None => {}
        }

        Ok(())
    }

    /// 撤销指定用户的所有有效 token
    async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<(), AppError> {
        let db = &*self.db;

        let models = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::Revoked.eq(false))
            .all(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        for model in models {
            let mut active: ActiveModel = model.into();
            active.revoked = sea_orm::Set(true);
            active
                .update(db)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        Ok(())
    }

    /// 删除过期的刷新令牌
    async fn delete_expired(&self) -> Result<u64, AppError> {
        let db = &*self.db;
        let result = Entity::delete_many()
            .filter(Column::ExpiresAt.lt(Utc::now()))
            .exec(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected)
    }
}
