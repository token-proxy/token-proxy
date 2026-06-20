//! 用户 API Key 仓储接口 — domain/user/
//!
//! 定义 `UserApiKeyRepository` trait，提供用户 API key 的持久化契约。

use crate::domain::user::UserApiKey;
use crate::shared::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

/// 用户 API Key 仓储接口
#[async_trait]
pub trait UserApiKeyRepository: Send + Sync {
    /// 根据 ID 查找 API key
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserApiKey>, AppError>;

    /// 根据 key_hash 查找 API key
    async fn find_by_key_hash(&self, key_hash: &str) -> Result<Option<UserApiKey>, AppError>;

    /// 查找指定用户的所有 API key
    async fn find_all_by_user(&self, user_id: Uuid) -> Result<Vec<UserApiKey>, AppError>;

    /// 保存 API key（创建或更新）
    async fn save(&self, key: &UserApiKey) -> Result<UserApiKey, AppError>;

    /// 撤销 API key
    async fn revoke(&self, id: Uuid) -> Result<(), AppError>;

    /// 更新最后使用时间
    async fn update_last_used(&self, id: Uuid) -> Result<(), AppError>;
}
