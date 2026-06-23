//! 服务商和账号仓储接口 — domain/provider/
//!
//! 定义 `ProviderRepository` 和 `AccountRepository` trait，
//! 提供服务商及其 API 账号的持久化契约。

use crate::domain::provider::account::Model as Account;
use crate::domain::provider::provider::Model as Provider;
use crate::shared::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

/// 服务商仓储接口
#[async_trait]
pub trait ProviderRepository: Send + Sync {
    /// 根据 ID 查找服务商
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Provider>, AppError>;
    /// 查询所有服务商
    async fn find_all(&self) -> Result<Vec<Provider>, AppError>;
    /// 查询所有已启用的服务商
    async fn find_enabled(&self) -> Result<Vec<Provider>, AppError>;
    /// 保存或更新服务商
    async fn save(&self, provider: &Provider) -> Result<Provider, AppError>;
    /// 删除服务商
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
}

/// API 账号仓储接口
#[async_trait]
pub trait AccountRepository: Send + Sync {
    /// 根据 ID 查找账号
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Account>, AppError>;
    /// 根据 Provider ID 查找关联的账号
    async fn find_by_provider_id(&self, provider_id: Uuid) -> Result<Vec<Account>, AppError>;
    /// 根据 Provider ID 查找已启用的账号
    async fn find_enabled_by_provider_id(
        &self,
        provider_id: Uuid,
    ) -> Result<Vec<Account>, AppError>;
    /// 获取账号的加密 API key
    async fn get_encrypted_api_key(&self, account_id: Uuid) -> Result<Vec<u8>, AppError>;
    /// 查询所有账号
    async fn find_all(&self) -> Result<Vec<Account>, AppError>;
    /// 保存或更新账号
    async fn save(&self, account: &Account) -> Result<Account, AppError>;
    /// 保存账号并设置加密的 API key
    async fn save_with_encrypted_key(
        &self,
        account: &Account,
        encrypted_api_key: &[u8],
    ) -> Result<Account, AppError>;
    /// 更新账号的加密 API key
    async fn update_encrypted_api_key(
        &self,
        account_id: Uuid,
        encrypted_api_key: &[u8],
    ) -> Result<(), AppError>;
    /// 删除账号
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
    /// 批量恢复已到恢复时间的自动禁用账号，返回实际恢复的账号 ID 列表
    async fn recover_expired_auto_disabled(&self) -> Result<Vec<Uuid>, AppError>;
}
