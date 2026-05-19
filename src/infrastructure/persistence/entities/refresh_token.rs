use sea_orm::entity::prelude::*;
use sea_orm::Set;

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

// ─── Related trait 实现 ───────────────────────────────────────────

impl Related<super::user::Entity> for Entity {
    fn to() -> sea_orm::RelationDef {
        Relation::User.def()
    }
}

// ─── 领域模型转换 ────────────────────────────────────────────────

use crate::domain::entities::refresh_token::RefreshToken;
use crate::shared::error::AppError;
use chrono::{FixedOffset, Utc};

impl TryFrom<Model> for RefreshToken {
    type Error = AppError;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        Ok(RefreshToken {
            id: model.id,
            user_id: model.user_id,
            token_hash: model.token_hash,
            expires_at: model.expires_at.with_timezone(&Utc),
            revoked: model.revoked,
            created_at: model.created_at.with_timezone(&Utc),
        })
    }
}

impl From<RefreshToken> for ActiveModel {
    fn from(token: RefreshToken) -> Self {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");

        ActiveModel {
            id: Set(token.id),
            user_id: Set(token.user_id),
            token_hash: Set(token.token_hash),
            expires_at: Set(token.expires_at.with_timezone(&offset)),
            revoked: Set(token.revoked),
            created_at: Set(token.created_at.with_timezone(&offset)),
        }
    }
}