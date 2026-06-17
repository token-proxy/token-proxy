use sea_orm::entity::prelude::*;

/// 系统设置（单行表，id 恒为 1）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "system_settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i16,
    pub log_retention_months: i16,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// 系统设置领域实体（从 ORM Model 解包）
#[derive(Clone, Debug)]
pub struct SystemSettings {
    pub log_retention_months: u32,
}

impl Default for SystemSettings {
    fn default() -> Self {
        SystemSettings {
            log_retention_months: 12,
        }
    }
}

impl SystemSettings {
    /// 从数据库行构造（解包 ORM 类型）
    pub fn from_model(model: &Model) -> Self {
        SystemSettings {
            log_retention_months: model.log_retention_months as u32,
        }
    }
}
