use crate::domain::entities::account::Account;
use crate::shared::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait AccountRepository: Send + Sync {
    /// 根据 ID 查找账号
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Account>, AppError>;

    /// 根据提供商 ID 查找所有关联账号
    async fn find_by_provider_id(&self, provider_id: Uuid) -> Result<Vec<Account>, AppError>;

    /// 根据提供商 ID 查找所有已启用的账号
    async fn find_enabled_by_provider_id(&self, provider_id: Uuid) -> Result<Vec<Account>, AppError>;

    /// 获取账号加密的 API Key 原始字节
    async fn get_encrypted_api_key(&self, account_id: Uuid) -> Result<Vec<u8>, AppError>;

    /// 查找所有账号
    async fn find_all(&self) -> Result<Vec<Account>, AppError>;

    /// 保存账号（创建或更新）
    async fn save(&self, account: &Account) -> Result<Account, AppError>;

    /// 创建账号并保存加密的 API Key
    async fn save_with_encrypted_key(
        &self,
        account: &Account,
        encrypted_api_key: &[u8],
    ) -> Result<Account, AppError>;

    /// 更新账号的加密 API Key
    async fn update_encrypted_api_key(
        &self,
        account_id: Uuid,
        encrypted_api_key: &[u8],
    ) -> Result<(), AppError>;

    /// 根据 ID 删除账号
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
}