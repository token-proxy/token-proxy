//! 系统设置实体 — domain/system/
//!
//! 定义 `SystemSettings` 领域实体和 SeaORM 实体映射 `system_settings` 表。
//! 单行表（id 恒为 1），使用 `SystemSettings` 领域类型解耦 ORM 实现。

use sea_orm::entity::prelude::*;

/// SeaORM 实体映射 system_settings 表（单行表，id 恒为 1）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "system_settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i16,
    pub log_retention_months: i16,
    pub log_storage_cap_gb: Option<i16>,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// 系统设置领域实体（从 ORM Model 解包）
#[derive(Clone, Debug)]
pub struct SystemSettings {
    /// 日志保留月数（默认 12，取值范围 1-36）
    pub log_retention_months: u32,
    /// 日志占用上限（GB），None 表示不限制
    pub log_storage_cap_gb: Option<u32>,
}

impl Default for SystemSettings {
    fn default() -> Self {
        SystemSettings {
            log_retention_months: 12,
            log_storage_cap_gb: None,
        }
    }
}

impl SystemSettings {
    /// 从数据库行构造（解包 ORM 类型）
    pub fn from_model(model: &Model) -> Self {
        SystemSettings {
            log_retention_months: model.log_retention_months as u32,
            log_storage_cap_gb: model.log_storage_cap_gb.map(|v| v as u32),
        }
    }
}
