use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};

use crate::domain::entities::user::User;
use crate::domain::repositories::user_repository::UserRepository;
use crate::domain::entities::user::{ActiveModel, Column, Entity};
use crate::shared::error::AppError;
use uuid::Uuid;

pub struct SeaOrmUserRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmUserRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        SeaOrmUserRepository { db }
    }
}

#[async_trait]
impl UserRepository for SeaOrmUserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AppError> {
        Ok(Entity::find_by_id(id).one(&*self.db).await?)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        Ok(Entity::find()
            .filter(Column::Username.eq(username))
            .one(&*self.db)
            .await?)
    }

    async fn exists_by_username(&self, username: &str) -> Result<bool, AppError> {
        Ok(Entity::find()
            .filter(Column::Username.eq(username))
            .count(&*self.db)
            .await? > 0)
    }

    async fn find_all(&self) -> Result<Vec<User>, AppError> {
        Ok(Entity::find().all(&*self.db).await?)
    }

    async fn save(&self, user: &User) -> Result<User, AppError> {
        let db = &*self.db;
        let exists = Entity::find_by_id(user.id).one(db).await?.is_some();

        let active_model: ActiveModel = user.clone().into();

        if exists {
            Entity::update(active_model).exec(db).await?;
        } else {
            Entity::insert(active_model).exec(db).await?;
        }

        Entity::find_by_id(user.id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Internal("保存后无法查询到 User".to_string()))
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        Entity::delete_by_id(id).exec(&*self.db).await?;
        Ok(())
    }
}
