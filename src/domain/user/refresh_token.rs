use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "refresh_tokens")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTimeWithTimeZone,
    pub revoked: bool,
    pub created_at: DateTimeWithTimeZone,

    #[sea_orm(belongs_to, from = "user_id", to = "id")]
    pub user: HasOne<super::user::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

// ─── 领域行为 ──────────────────────────────────────────────────────

impl Model {
    pub fn new(user_id: Uuid, token_hash: String, expires_at: DateTime<Utc>) -> Self {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        Model {
            id: Uuid::new_v4(),
            user_id,
            token_hash,
            expires_at: expires_at.with_timezone(&offset),
            revoked: false,
            created_at: Utc::now().with_timezone(&offset),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at.with_timezone(&Utc)
    }

    pub fn is_valid(&self) -> bool {
        !self.revoked && !self.is_expired()
    }

    pub fn expires_at_utc(&self) -> DateTime<Utc> {
        self.expires_at.with_timezone(&Utc)
    }

    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.with_timezone(&Utc)
    }
}
