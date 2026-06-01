use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// SeaORM 实体映射 refresh_tokens 表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "refresh_tokens")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTimeWithTimeZone,
    pub revoked: bool,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::user::Entity> for Entity {
    fn to() -> sea_orm::RelationDef {
        Relation::User.def()
    }
}

/// 领域实体 RefreshToken
pub type RefreshToken = Model;

// ─── 领域行为 ──────────────────────────────────────────────────────

impl Model {
    /// 创建新的 RefreshToken
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

    /// 判断 token 是否已过期
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at.with_timezone(&Utc)
    }

    /// 判断 token 是否有效（未撤销且未过期）
    pub fn is_valid(&self) -> bool {
        !self.revoked && !self.is_expired()
    }

    /// 获取 expires_at 为 DateTime<Utc>
    pub fn expires_at_utc(&self) -> DateTime<Utc> {
        self.expires_at.with_timezone(&Utc)
    }

    /// 获取 created_at 为 DateTime<Utc>
    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.with_timezone(&Utc)
    }
}
