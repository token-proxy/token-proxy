use crate::domain::user::RefreshToken;
use crate::shared::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait RefreshTokenRepository: Send + Sync {
    /// 根据 ID 查找刷新令牌
    async fn find_by_id(&self, id: Uuid) -> Result<Option<RefreshToken>, AppError>;

    /// 根据 token_hash 查找刷新令牌
    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<RefreshToken>, AppError>;

    /// 查找用户所有有效（未撤销且未过期）的刷新令牌
    async fn find_valid_by_user_id(&self, user_id: Uuid) -> Result<Vec<RefreshToken>, AppError>;

    /// 保存刷新令牌
    async fn save(&self, token: &RefreshToken) -> Result<RefreshToken, AppError>;

    /// 撤销指定的刷新令牌
    async fn revoke(&self, id: Uuid) -> Result<(), AppError>;

    /// 撤销用户的所有刷新令牌
    async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<(), AppError>;

    /// 删除过期的刷新令牌
    async fn delete_expired(&self) -> Result<u64, AppError>;
}
