use sea_orm::entity::prelude::*;
use sea_orm::Set;

/// SeaORM 实体映射 providers 表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "providers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
    #[sea_orm(column_type = "JsonBinary")]
    pub models: Json,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::account::Entity")]
    Account,
}

impl ActiveModelBehavior for ActiveModel {}

// ─── 领域模型转换 ────────────────────────────────────────────────

use crate::domain::entities::provider::Provider;
use crate::domain::value_objects::status::Status;
use crate::shared::error::AppError;
use chrono::{FixedOffset, Utc};
use std::str::FromStr;

impl TryFrom<Model> for Provider {
    type Error = AppError;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        let models: Vec<String> = serde_json::from_value(model.models)
            .map_err(|e| AppError::Database(format!("解析 models JSON 失败: {}", e)))?;

        Ok(Provider {
            id: model.id,
            name: model.name,
            openai_base_url: model.openai_base_url,
            anthropic_base_url: model.anthropic_base_url,
            models,
            status: Status::from_str(&model.status)?,
            created_at: model.created_at.with_timezone(&Utc),
            updated_at: model.updated_at.with_timezone(&Utc),
        })
    }
}

impl From<Provider> for ActiveModel {
    fn from(provider: Provider) -> Self {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");

        ActiveModel {
            id: Set(provider.id),
            name: Set(provider.name),
            openai_base_url: Set(provider.openai_base_url),
            anthropic_base_url: Set(provider.anthropic_base_url),
            models: Set(
                serde_json::to_value(&provider.models)
                    .unwrap_or(serde_json::Value::Array(vec![])),
            ),
            status: Set(provider.status.to_string()),
            created_at: Set(provider.created_at.with_timezone(&offset)),
            updated_at: Set(provider.updated_at.with_timezone(&offset)),
        }
    }
}