use sea_orm::entity::prelude::*;
use sea_orm::Set;

/// SeaORM 实体映射 log_contents 表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "log_contents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub log_id: Uuid,
    pub request_headers: Option<Json>,
    pub request_body: Option<Json>,
    pub response_body: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ─── 领域模型转换 ────────────────────────────────────────────────

use crate::domain::entities::log_entry::LogContent;
use crate::shared::error::AppError;

impl TryFrom<Model> for LogContent {
    type Error = AppError;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        Ok(LogContent {
            log_id: model.log_id,
            request_headers: model.request_headers.unwrap_or(serde_json::Value::Null),
            request_body: model.request_body.unwrap_or(serde_json::Value::Null),
            response_body: model.response_body.unwrap_or_default(),
        })
    }
}

impl From<LogContent> for ActiveModel {
    fn from(content: LogContent) -> Self {
        ActiveModel {
            log_id: Set(content.log_id),
            request_headers: Set(Some(content.request_headers)),
            request_body: Set(Some(content.request_body)),
            response_body: Set(Some(content.response_body)),
        }
    }
}