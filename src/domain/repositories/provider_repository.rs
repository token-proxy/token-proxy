use crate::domain::entities::provider::Provider;
use crate::shared::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait ProviderRepository: Send + Sync {
    /// 根据 ID 查找提供商
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Provider>, AppError>;

    /// 查找所有提供商
    async fn find_all(&self) -> Result<Vec<Provider>, AppError>;

    /// 查找所有已启用的提供商
    async fn find_enabled(&self) -> Result<Vec<Provider>, AppError>;

    /// 保存提供商（创建或更新）
    async fn save(&self, provider: &Provider) -> Result<Provider, AppError>;

    /// 根据 ID 删除提供商
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
}