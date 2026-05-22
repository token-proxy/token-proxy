use std::sync::Arc;

use uuid::Uuid;

use crate::application::dto::account_dto::{
    AccountResponse, CreateAccountRequest, UpdateAccountRequest,
};
use crate::domain::entities::account::Account;
use crate::domain::repositories::account_repository::AccountRepository;
use crate::domain::repositories::provider_repository::ProviderRepository;
use crate::domain::services::encryption_service::EncryptionService;
use crate::domain::value_objects::api_key::ApiKey;
use crate::shared::error::AppError;

pub struct AccountService {
    account_repo: Arc<dyn AccountRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    encryption_service: Arc<dyn EncryptionService>,
}

impl AccountService {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        encryption_service: Arc<dyn EncryptionService>,
    ) -> Self {
        AccountService {
            account_repo,
            provider_repo,
            encryption_service,
        }
    }

    fn to_response(account: &Account) -> AccountResponse {
        AccountResponse {
            id: account.id,
            provider_id: account.provider_id,
            name: account.name.clone(),
            api_key_suffix: account.api_key_suffix.clone(),
            status: account.status.to_string(),
            created_at: account.created_at,
            updated_at: account.updated_at,
        }
    }

    pub async fn create(
        &self,
        provider_id: Uuid,
        req: CreateAccountRequest,
    ) -> Result<AccountResponse, AppError> {
        // 检查 Provider 是否存在
        let provider = self
            .provider_repo
            .find_by_id(provider_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("提供商 {} 未找到", provider_id)))?;

        if !provider.status.is_enabled() {
            return Err(AppError::Conflict(
                "关联的提供商已被禁用，无法创建账号".to_string(),
            ));
        }

        let raw_key = req.api_key.trim().to_string();
        if raw_key.is_empty() {
            return Err(AppError::Validation("API Key 不能为空".to_string()));
        }

        let api_key = ApiKey::new(raw_key);
        let suffix = api_key.suffix();

        // 加密 API Key
        let encrypted = self
            .encryption_service
            .encrypt(api_key.as_str().as_bytes())
            .await
            .map_err(|e| AppError::Encryption(e.to_string()))?;

        let name = req
            .name
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("account_{}", suffix));

        let account = Account::new(provider_id, name, suffix);

        let saved = self
            .account_repo
            .save_with_encrypted_key(&account, &encrypted)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Self::to_response(&saved))
    }

    pub async fn update(
        &self,
        id: Uuid,
        req: UpdateAccountRequest,
    ) -> Result<AccountResponse, AppError> {
        let mut account = self
            .account_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("账号 {} 未找到", id)))?;

        if let Some(name) = req.name {
            let trimmed = name.trim().to_string();
            if trimmed.is_empty() {
                return Err(AppError::Validation("账号名称不能为空".to_string()));
            }
            account.name = trimmed;
        }

        // 如果客户端提供了新的 API Key，更新加密 Key 与后缀
        if let Some(raw_key) = req.api_key {
            let trimmed = raw_key.trim().to_string();
            if !trimmed.is_empty() {
                let api_key = ApiKey::new(trimmed);
                account.api_key_suffix = api_key.suffix();

                let encrypted = self
                    .encryption_service
                    .encrypt(api_key.as_str().as_bytes())
                    .await
                    .map_err(|e| AppError::Encryption(e.to_string()))?;

                self.account_repo
                    .update_encrypted_api_key(id, &encrypted)
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;
            }
        }

        account.updated_at = chrono::Utc::now();

        let saved = self
            .account_repo
            .save(&account)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Self::to_response(&saved))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<AccountResponse, AppError> {
        let account = self
            .account_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("账号 {} 未找到", id)))?;

        Ok(Self::to_response(&account))
    }

    pub async fn list_by_provider(
        &self,
        provider_id: Uuid,
    ) -> Result<Vec<AccountResponse>, AppError> {
        let accounts = self
            .account_repo
            .find_by_provider_id(provider_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(accounts.iter().map(Self::to_response).collect())
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        self.account_repo
            .delete(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}