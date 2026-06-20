//! 刷新令牌实体 — domain/user/
//!
//! 定义 `RefreshToken`（SeaORM 实体映射 `refresh_tokens` 表），
//! 提供过期检查、有效性验证等领域行为。

use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// SeaORM 实体映射 refresh_tokens 表
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "refresh_tokens")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    /// token 哈希值（非明文存储）
    pub token_hash: String,
    pub expires_at: DateTimeWithTimeZone,
    /// 是否已撤销
    pub revoked: bool,
    pub created_at: DateTimeWithTimeZone,

    #[sea_orm(belongs_to, from = "user_id", to = "id")]
    pub user: HasOne<super::user::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

// ─── 领域行为 ──────────────────────────────────────────────────────

impl Model {
    /// 创建新的刷新令牌，初始为未撤销状态
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

    /// 判断是否已过期
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at.with_timezone(&Utc)
    }

    /// 判断是否有效（未撤销且未过期）
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
