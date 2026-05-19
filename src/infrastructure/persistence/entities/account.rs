use sea_orm::entity::prelude::*;

/// SeaORM 实体映射 accounts 表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "accounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub provider_id: Uuid,
    pub name: String,
    pub api_key_encrypted: Vec<u8>,
    pub api_key_suffix: String,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::provider::Entity",
        from = "Column::ProviderId",
        to = "super::provider::Column::Id"
    )]
    Provider,
}

impl ActiveModelBehavior for ActiveModel {}

// ─── Related trait 实现 ───────────────────────────────────────────

impl Related<super::provider::Entity> for Entity {
    fn to() -> sea_orm::RelationDef {
        Relation::Provider.def()
    }
}

// ─── 领域模型转换 ────────────────────────────────────────────────

use crate::domain::entities::account::Account;
use crate::domain::value_objects::status::Status;
use crate::shared::error::AppError;
use chrono::Utc;
use std::str::FromStr;

impl TryFrom<Model> for Account {
    type Error = AppError;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        Ok(Account {
            id: model.id,
            provider_id: model.provider_id,
            name: model.name,
            api_key_suffix: model.api_key_suffix,
            status: Status::from_str(&model.status)?,
            created_at: model.created_at.with_timezone(&Utc),
            updated_at: model.updated_at.with_timezone(&Utc),
        })
    }
}