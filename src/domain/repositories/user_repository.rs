use crate::domain::entities::user::User;
use crate::shared::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait UserRepository: Send + Sync {
    /// 根据 ID 查找用户
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AppError>;

    /// 根据用户名查找用户
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError>;

    /// 查找所有用户
    async fn find_all(&self) -> Result<Vec<User>, AppError>;

    /// 保存用户（创建或更新）
    async fn save(&self, user: &User) -> Result<User, AppError>;

    /// 根据 ID 删除用户
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;

    /// 检查用户名是否已存在
    async fn exists_by_username(&self, username: &str) -> Result<bool, AppError>;
}
