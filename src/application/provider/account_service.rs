//! 账号应用服务 — application/provider/
//!
//! 编排 Provider 账号的 CRUD 操作，包括 API Key 加密存储、
//! 账号禁用/恢复、以及删除时自动从接入点账户池中移除。

use std::sync::Arc;

use tracing;
use uuid::Uuid;

use super::dto::{AccountResponse, CreateAccountRequest, UpdateAccountRequest};
use crate::domain::access_point::repository::AccessPointRepository;
use crate::domain::provider::repository::AccountRepository;
use crate::domain::provider::repository::ProviderRepository;
use crate::domain::provider::Account;
use crate::domain::shared::ApiKey;
use crate::domain::shared::EncryptionService;
use crate::domain::shared::Status;
use crate::shared::error::AppError;

/// 账号应用服务
///
/// 编排 Provider 账号的 CRUD、加密、禁用/恢复操作。
pub struct AccountService {
    account_repo: Arc<dyn AccountRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    access_point_repo: Arc<dyn AccessPointRepository>,
    encryption_service: Arc<dyn EncryptionService>,
}

impl AccountService {
    pub fn new(
        account_repo: Arc<dyn AccountRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        access_point_repo: Arc<dyn AccessPointRepository>,
        encryption_service: Arc<dyn EncryptionService>,
    ) -> Self {
        AccountService {
            account_repo,
            provider_repo,
            access_point_repo,
            encryption_service,
        }
    }

    fn to_response(account: &Account) -> AccountResponse {
        AccountResponse {
            id: account.id,
            provider_id: account.provider_id,
            name: account.name.clone(),
            api_key_suffix: account.api_key_suffix.clone(),
            disabled_reason: account.disabled_reason.as_ref().map(|r| r.to_string()),
            available_at: account.available_at.map(|dt| dt.with_timezone(&chrono::Utc)),
            status: account.status.to_string(),
            created_at: account.created_at.with_timezone(&chrono::Utc),
            updated_at: account.updated_at.with_timezone(&chrono::Utc),
        }
    }

    /// 创建账号
    ///
    /// 校验 Provider 存在且启用，加密 API Key，保存账号记录。
    pub async fn create(
        &self,
        provider_id: Uuid,
        req: CreateAccountRequest,
    ) -> Result<AccountResponse, AppError> {
        // 检查 Provider 是否存在
        let provider = self
            .provider_repo
            .find_by_id(provider_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("服务商 {} 未找到", provider_id)))?;

        if !provider.status.is_enabled() {
            return Err(AppError::Conflict(
                "关联的服务商已被禁用，无法创建账号".to_string(),
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
            .await?;

        Ok(Self::to_response(&saved))
    }

    /// 更新账号
    ///
    /// 支持更新名称和 API Key（提供时触发重新加密）。
    pub async fn update(
        &self,
        id: Uuid,
        req: UpdateAccountRequest,
    ) -> Result<AccountResponse, AppError> {
        let mut account = self
            .account_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("账号 {} 未找到", id)))?;

        if let Some(name) = req.name {
            account.set_name(name)?;
        }

        // 如果客户端提供了新的 API Key，更新加密 Key 与后缀
        if let Some(raw_key) = req.api_key {
            let trimmed = raw_key.trim().to_string();
            if !trimmed.is_empty() {
                let api_key = ApiKey::new(trimmed);
                account.update_api_key_suffix(api_key.suffix());

                let encrypted = self
                    .encryption_service
                    .encrypt(api_key.as_str().as_bytes())
                    .await
                    .map_err(|e| AppError::Encryption(e.to_string()))?;

                self.account_repo
                    .update_encrypted_api_key(id, &encrypted)
                    .await?;
            }
        }

        let saved = self.account_repo.save(&account).await?;

        Ok(Self::to_response(&saved))
    }

    /// 根据 ID 查询账号
    pub async fn get_by_id(&self, id: Uuid) -> Result<AccountResponse, AppError> {
        let account = self
            .account_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("账号 {} 未找到", id)))?;

        Ok(Self::to_response(&account))
    }

    /// 查询指定 Provider 的所有账号
    pub async fn list_by_provider(
        &self,
        provider_id: Uuid,
    ) -> Result<Vec<AccountResponse>, AppError> {
        let accounts = self.account_repo.find_by_provider_id(provider_id).await?;

        Ok(accounts.iter().map(Self::to_response).collect())
    }

    /// 删除账号，同时从所有关联接入点的账户池中移除该账号
    pub async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        // 1. 查找引用此账号的所有接入点
        let affected_aps = self.access_point_repo.find_by_account_id(id).await?;

        // 2. 从每个接入点的账户池中移除此账号
        for ap in &affected_aps {
            let accounts = self
                .access_point_repo
                .find_accounts_by_access_point(ap.id)
                .await?;
            let filtered: Vec<_> = accounts.into_iter().filter(|a| a.account_id != id).collect();
            self.access_point_repo
                .save_accounts(ap.id, &filtered)
                .await?;
            tracing::info!(
                account_id = %id,
                access_point_id = %ap.id,
                "已从接入点账户池中移除账号",
            );
        }

        // 3. 删除账号本身
        self.account_repo.delete(id).await?;

        Ok(())
    }

    /// 恢复账号（清除 disabled_reason 和 available_at，重置为启用状态）
    pub async fn recover(&self, id: Uuid) -> Result<AccountResponse, AppError> {
        let mut account = self
            .account_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("账号 {} 未找到", id)))?;

        account.recover();

        let saved = self.account_repo.save(&account).await?;

        Ok(Self::to_response(&saved))
    }

    /// 设置账号状态
    ///
    /// 委托给 `Account::set_status()` 处理状态转换规则。
    pub async fn set_status(&self, id: Uuid, status: Status) -> Result<AccountResponse, AppError> {
        let mut account = self
            .account_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("账号 {} 未找到", id)))?;

        account.set_status(status);

        let saved = self.account_repo.save(&account).await?;

        Ok(Self::to_response(&saved))
    }
}
