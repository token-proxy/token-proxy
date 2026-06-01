use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::value_objects::status::Status;

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
    pub status: Status,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ─── 类型别名 ──────────────────────────────────────────────────────
//
// 保持 `domain::entities::user_api_key::UserApiKey` 导入路径不变

/// 领域实体 UserApiKey
pub type UserApiKey = Model;

// ─── 领域行为 ──────────────────────────────────────────────────────

impl Model {
    /// 创建新的 UserApiKey
    pub fn new(user_id: Uuid, key_hash: String, key_prefix: String, description: String) -> Self {
        let now = Utc::now();
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        Model {
            id: Uuid::new_v4(),
            user_id,
            key_hash,
            key_prefix,
            description,
            last_used_at: None,
            status: Status::Enabled,
            created_at: now.with_timezone(&offset),
        }
    }

    /// 获取 created_at 为 DateTime<Utc>
    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.with_timezone(&Utc)
    }

    /// 获取 last_used_at 为 Option<DateTime<Utc>>
    pub fn last_used_at_utc(&self) -> Option<DateTime<Utc>> {
        self.last_used_at.map(|dt| dt.with_timezone(&Utc))
    }
}
