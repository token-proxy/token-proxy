use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::value_objects::status::Status;

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
    pub status: Status,
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

impl Related<super::provider::Entity> for Entity {
    fn to() -> sea_orm::RelationDef {
        Relation::Provider.def()
    }
}

/// 领域实体 Account
pub type Account = Model;

// ─── 领域行为 ──────────────────────────────────────────────────────

impl Model {
    /// 创建新的 Account，仅存储 API Key 末尾 6 位
    pub fn new(provider_id: Uuid, name: String, api_key_suffix: String) -> Self {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        let now = Utc::now().with_timezone(&offset);
        Account {
            id: Uuid::new_v4(),
            provider_id,
            name,
            api_key_encrypted: Vec::new(),
            api_key_suffix,
            status: Status::Enabled,
            created_at: now,
            updated_at: now,
        }
    }

    /// 获取 created_at 为 DateTime<Utc>
    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.with_timezone(&Utc)
    }

    /// 获取 updated_at 为 DateTime<Utc>
    pub fn updated_at_utc(&self) -> DateTime<Utc> {
        self.updated_at.with_timezone(&Utc)
    }
}
