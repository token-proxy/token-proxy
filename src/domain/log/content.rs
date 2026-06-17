use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// SeaORM 实体映射 log_contents 表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "log_contents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub log_id: Uuid,
    pub request_headers: Option<Json>,
    pub request_body: Option<Json>,
    pub response_body: Option<String>,
    pub response_headers: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
