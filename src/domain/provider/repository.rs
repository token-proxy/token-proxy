use crate::domain::provider::account::Model as Account;
use crate::domain::provider::provider::Model as Provider;
use crate::shared::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait ProviderRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Provider>, AppError>;
    async fn find_all(&self) -> Result<Vec<Provider>, AppError>;
    async fn find_enabled(&self) -> Result<Vec<Provider>, AppError>;
    async fn save(&self, provider: &Provider) -> Result<Provider, AppError>;
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
}

#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Account>, AppError>;
    async fn find_by_provider_id(&self, provider_id: Uuid) -> Result<Vec<Account>, AppError>;
    async fn find_enabled_by_provider_id(
        &self,
        provider_id: Uuid,
    ) -> Result<Vec<Account>, AppError>;
    async fn get_encrypted_api_key(&self, account_id: Uuid) -> Result<Vec<u8>, AppError>;
    async fn find_all(&self) -> Result<Vec<Account>, AppError>;
    async fn save(&self, account: &Account) -> Result<Account, AppError>;
    async fn save_with_encrypted_key(
        &self,
        account: &Account,
        encrypted_api_key: &[u8],
    ) -> Result<Account, AppError>;
    async fn update_encrypted_api_key(
        &self,
        account_id: Uuid,
        encrypted_api_key: &[u8],
    ) -> Result<(), AppError>;
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
}
