use sea_orm::entity::prelude::*;
use sea_orm::Set;

/// SeaORM 实体映射 users 表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::refresh_token::Entity")]
    RefreshToken,

    #[sea_orm(has_many = "super::access_point::Entity")]
    AccessPoint,
}

impl ActiveModelBehavior for ActiveModel {}

// ─── 领域模型转换 ────────────────────────────────────────────────

use crate::domain::entities::user::User;
use crate::domain::value_objects::status::Status;
use crate::shared::error::AppError;
use chrono::{FixedOffset, Utc};
use std::str::FromStr;

impl TryFrom<Model> for User {
    type Error = AppError;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        Ok(User {
            id: model.id,
            username: model.username,
            display_name: model.display_name,
            password_hash: model.password_hash,
            status: Status::from_str(&model.status)?,
            created_at: model.created_at.with_timezone(&Utc),
            updated_at: model.updated_at.with_timezone(&Utc),
        })
    }
}

impl From<User> for ActiveModel {
    fn from(user: User) -> Self {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");

        ActiveModel {
            id: Set(user.id),
            username: Set(user.username),
            display_name: Set(user.display_name),
            password_hash: Set(user.password_hash),
            status: Set(user.status.to_string()),
            created_at: Set(user.created_at.with_timezone(&offset)),
            updated_at: Set(user.updated_at.with_timezone(&offset)),
        }
    }
}