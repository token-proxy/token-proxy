//! 用户实体 — domain/user/
//!
//! 定义 `User`（SeaORM 实体映射 `users` 表），
//! 包含用户名、密码哈希、展示名称和状态。

use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::shared::status::Status;
use crate::shared::error::AppError;

/// SeaORM 实体映射 users 表
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub status: Status,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,

    #[sea_orm(has_many)]
    pub refresh_tokens: HasMany<super::refresh_token::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// 创建新用户，自动生成 ID 和时间戳
    pub fn new(username: String, display_name: String, password_hash: String) -> Self {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        let now = Utc::now().with_timezone(&offset);
        Model {
            id: Uuid::new_v4(),
            username,
            display_name,
            password_hash,
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

    /// 设置展示名称，名称不可为空
    pub fn set_display_name(&mut self, name: String) -> Result<(), AppError> {
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            return Err(AppError::Validation("显示名称不能为空".to_string()));
        }
        self.display_name = trimmed;
        self.touch();
        Ok(())
    }

    /// 设置密码哈希
    pub fn set_password_hash(&mut self, hash: String) {
        self.password_hash = hash;
        self.touch();
    }

    /// 启用用户
    pub fn enable(&mut self) {
        self.status = Status::Enabled;
        self.touch();
    }

    /// 禁用用户
    pub fn disable(&mut self) {
        self.status = Status::Disabled;
        self.touch();
    }

    /// 校验用户处于启用状态
    ///
    /// 禁用用户不得再通过任何入口访问：面板登录、token 刷新、JWT 接口（含 SSE）
    /// 与 API key 代理调用都在各自入口调用本方法，避免"禁用只是状态位、权限没断"。
    pub fn ensure_enabled(&self) -> Result<(), AppError> {
        if self.status.is_enabled() {
            Ok(())
        } else {
            Err(AppError::Forbidden("用户已被禁用".to_string()))
        }
    }

    /// 更新 updated_at 为当前时间
    fn touch(&mut self) {
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        self.updated_at = chrono::Utc::now().with_timezone(&offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_with(status: Status) -> Model {
        let mut user = Model::new("u".to_string(), "U".to_string(), "hash".to_string());
        user.status = status;
        user
    }

    #[test]
    fn test_ensure_enabled_allows_enabled_user() {
        assert!(user_with(Status::Enabled).ensure_enabled().is_ok());
    }

    #[test]
    fn test_ensure_enabled_rejects_disabled_user() {
        let err = user_with(Status::Disabled).ensure_enabled().unwrap_err();
        assert!(matches!(err, AppError::Forbidden(_)), "应为 403 Forbidden");
        assert!(err.to_string().contains("禁用"));
    }

    #[test]
    fn test_disable_then_ensure_enabled_rejected() {
        let mut user = user_with(Status::Enabled);
        user.disable();
        assert!(user.ensure_enabled().is_err());
        user.enable();
        assert!(user.ensure_enabled().is_ok());
    }
}
