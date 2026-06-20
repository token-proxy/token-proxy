//! 接入点-账号关联 Repository 实现（基础设施层）
//!
//! `access_point_accounts` 表实体的 SeaORM 定义。
//! 此表为 AccessPoint 与 Account 的多对多关联表。

use sea_orm::entity::prelude::*;
use uuid::Uuid;

// ─── SeaORM 实体 (access_point_accounts 表) ───────────────────────

/// 接入点-账号关联记录
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "access_point_accounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub access_point_id: Uuid,
    pub account_id: Uuid,
    pub weight: i32,
    pub priority: i32,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::domain::access_point::access_point::Entity",
        from = "Column::AccessPointId",
        to = "crate::domain::access_point::access_point::Column::Id"
    )]
    AccessPoint,
    #[sea_orm(
        belongs_to = "crate::domain::provider::account::Entity",
        from = "Column::AccountId",
        to = "crate::domain::provider::account::Column::Id"
    )]
    Account,
}

impl ActiveModelBehavior for ActiveModel {}
