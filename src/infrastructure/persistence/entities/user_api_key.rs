use sea_orm::entity::prelude::*;
use sea_orm::Set;

/// SeaORM 实体映射 user_api_keys 表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user_api_keys")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    #[sea_orm(unique)]
    pub key_hash: String,
    pub key_prefix: String,
    pub description: String,
    pub last_used_at: Option<DateTimeWithTimeZone>,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ─── 领域模型转换 ────────────────────────────────────────────────

use crate::domain::entities::user_api_key::UserApiKey;
use crate::domain::value_objects::status::Status;
use crate::shared::error::AppError;
use chrono::{FixedOffset, Utc};
use std::str::FromStr;

impl TryFrom<Model> for UserApiKey {
    type Error = AppError;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        Ok(UserApiKey {
            id: model.id,
            user_id: model.user_id,
            key_hash: model.key_hash,
            key_prefix: model.key_prefix,
            description: model.description,
            last_used_at: model.last_used_at.map(|dt| dt.with_timezone(&Utc)),
            status: Status::from_str(&model.status)?,
            created_at: model.created_at.with_timezone(&Utc),
        })
    }
}

impl From<UserApiKey> for ActiveModel {
    fn from(key: UserApiKey) -> Self {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");

        ActiveModel {
            id: Set(key.id),
            user_id: Set(key.user_id),
            key_hash: Set(key.key_hash),
            key_prefix: Set(key.key_prefix),
            description: Set(key.description),
            last_used_at: Set(key.last_used_at.map(|dt| dt.with_timezone(&offset))),
            status: Set(key.status.to_string()),
            created_at: Set(key.created_at.with_timezone(&offset)),
        }
    }
}