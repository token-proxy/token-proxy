use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::shared::status::Status;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
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

    #[sea_orm(belongs_to, from = "provider_id", to = "id")]
    pub provider: HasOne<super::provider::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn new(provider_id: Uuid, name: String, api_key_suffix: String) -> Self {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        let now = Utc::now().with_timezone(&offset);
        Model {
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

    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.with_timezone(&Utc)
    }

    pub fn updated_at_utc(&self) -> DateTime<Utc> {
        self.updated_at.with_timezone(&Utc)
    }
}
