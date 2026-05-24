use sea_orm::entity::prelude::*;
use sea_orm::Set;

/// SeaORM 实体映射 access_points 表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "access_points")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub api_type: String,
    #[sea_orm(unique)]
    pub short_code: String,
    pub provider_id: Uuid,
    pub account_id: Uuid,
    #[sea_orm(column_type = "JsonBinary")]
    pub model_mappings: Json,
    pub status: String,
    pub created_by: Uuid,
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

    #[sea_orm(
        belongs_to = "super::account::Entity",
        from = "Column::AccountId",
        to = "super::account::Column::Id"
    )]
    Account,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    User,
}

impl ActiveModelBehavior for ActiveModel {}

// ─── Related trait 实现 ───────────────────────────────────────────

impl Related<super::user::Entity> for Entity {
    fn to() -> sea_orm::RelationDef {
        Relation::User.def()
    }
}

impl Related<super::provider::Entity> for Entity {
    fn to() -> sea_orm::RelationDef {
        Relation::Provider.def()
    }
}

impl Related<super::account::Entity> for Entity {
    fn to() -> sea_orm::RelationDef {
        Relation::Account.def()
    }
}

// ─── 领域模型转换 ────────────────────────────────────────────────

use crate::domain::entities::access_point::AccessPoint;
use crate::domain::value_objects::access_point_type::AccessPointType;
use crate::domain::value_objects::model_mapping::ModelMapping;
use crate::domain::value_objects::short_code::ShortCode;
use crate::domain::value_objects::status::Status;
use crate::shared::error::AppError;
use chrono::{FixedOffset, Utc};
use std::str::FromStr;

impl TryFrom<Model> for AccessPoint {
    type Error = AppError;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        let model_mappings: Vec<ModelMapping> = serde_json::from_value(model.model_mappings)
            .map_err(|e| AppError::Database(format!("解析 model_mappings JSON 失败: {}", e)))?;

        Ok(AccessPoint {
            id: model.id,
            name: model.name,
            api_type: AccessPointType::from_str(&model.api_type)?,
            short_code: ShortCode::new(&model.short_code)?,
            provider_id: model.provider_id,
            account_id: model.account_id,
            model_mappings,
            status: Status::from_str(&model.status)?,
            created_by: model.created_by,
            created_at: model.created_at.with_timezone(&Utc),
            updated_at: model.updated_at.with_timezone(&Utc),
        })
    }
}

impl From<AccessPoint> for ActiveModel {
    fn from(ap: AccessPoint) -> Self {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");

        ActiveModel {
            id: Set(ap.id),
            name: Set(ap.name),
            api_type: Set(ap.api_type.to_string()),
            short_code: Set(ap.short_code.to_string()),
            provider_id: Set(ap.provider_id),
            account_id: Set(ap.account_id),
            model_mappings: Set(serde_json::to_value(&ap.model_mappings)
                .unwrap_or(serde_json::Value::Array(vec![]))),
            status: Set(ap.status.to_string()),
            created_by: Set(ap.created_by),
            created_at: Set(ap.created_at.with_timezone(&offset)),
            updated_at: Set(ap.updated_at.with_timezone(&offset)),
        }
    }
}
