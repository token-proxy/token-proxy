use std::sync::Arc;

use crate::domain::entities::access_point::AccessPoint;
use crate::domain::entities::account::Account;
use crate::domain::entities::provider::Provider;
use crate::domain::repositories::access_point_repository::AccessPointRepository;
use crate::domain::repositories::account_repository::AccountRepository;
use crate::domain::repositories::provider_repository::ProviderRepository;
use crate::domain::services::encryption_service::EncryptionService;
use crate::shared::error::AppError;

/// 代理上下文，包含解析后的接入点、提供商、账号和解密的 API Key
#[derive(Debug, Clone)]
pub struct ProxyContext {
    pub access_point: AccessPoint,
    pub provider: Provider,
    pub account: Account,
    pub decrypted_api_key: String,
}

pub struct ProxyService {
    access_point_repo: Arc<dyn AccessPointRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    account_repo: Arc<dyn AccountRepository>,
    #[allow(dead_code)]
    encryption_service: Arc<dyn EncryptionService>,
}

impl ProxyService {
    pub fn new(
        access_point_repo: Arc<dyn AccessPointRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        account_repo: Arc<dyn AccountRepository>,
        encryption_service: Arc<dyn EncryptionService>,
    ) -> Self {
        ProxyService {
            access_point_repo,
            provider_repo,
            account_repo,
            encryption_service,
        }
    }

    /// 根据短码解析代理上下文
    ///
    /// 依次查找并校验:
    /// 1. AccessPoint: 按 short_code 查找，校验 status 为 Enabled
    /// 2. Provider: 按 access_point.provider_id 查找，校验 status 为 Enabled
    /// 3. Account: 按 access_point.account_id 查找，校验 status 为 Enabled，解密 api_key
    /// 4. 返回 ProxyContext
    pub async fn resolve_context(&self, short_code: &str) -> Result<ProxyContext, AppError> {
        // 1. 查找 AccessPoint
        let access_point = self
            .access_point_repo
            .find_by_short_code(short_code)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("接入点 '{}' 未找到", short_code)))?;

        if !access_point.status.is_enabled() {
            return Err(AppError::Forbidden(format!(
                "接入点 '{}' 已被禁用",
                short_code
            )));
        }

        // 2. 查找 Provider
        let provider = self
            .provider_repo
            .find_by_id(access_point.provider_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "接入点 '{}' 关联的提供商未找到",
                    short_code
                ))
            })?;

        if !provider.status.is_enabled() {
            return Err(AppError::Forbidden(format!(
                "接入点 '{}' 关联的提供商已被禁用",
                short_code
            )));
        }

        // 3. 查找 Account 并解密 API Key
        let account = self
            .account_repo
            .find_by_id(access_point.account_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "接入点 '{}' 关联的账号未找到",
                    short_code
                ))
            })?;

        if !account.status.is_enabled() {
            return Err(AppError::Forbidden(format!(
                "接入点 '{}' 关联的账号已被禁用",
                short_code
            )));
        }

        // 解密 API Key
        // 注意: 此处假设 account 实体在 Infra 层持久化时保存了 encrypted_api_key 字段。
        // 实际解密逻辑依赖于 SeaOrmAccountRepository 如何存储和提供加密数据。
        // 这里通过缓存/关联查询获取已加密的 api_key 密文（base64 编码）
        // 解密流程由 Infra 层的 AccountRepository 实现配合完成
        let decrypted_api_key = self
            .decrypt_account_key(&account)
            .await?;

        Ok(ProxyContext {
            access_point,
            provider,
            account,
            decrypted_api_key,
        })
    }

    /// 解密账号的 API Key
    ///
    /// 该方法需要从 Account 实体或其关联数据中获取加密的 API Key，
    /// 然后使用 encryption_service 解密。
    ///
    /// 由于 Account 实体本身只有 api_key_suffix，没有 encrypted_api_key 字段，
    /// 实际的解密流程会在 SeaOrmAccountRepository 中实现。
    /// 此处仅作为 Service 层的编排接口，具体的解密由 Infra 层配合完成。
    async fn decrypt_account_key(&self, _account: &Account) -> Result<String, AppError> {
        // 此方法实际由 Infra 层 AccountRepository 的实现提供解密能力
        // 在后续开发中，这里会从 repository 获取加密的 api_key 并调用 encryption_service.decrypt
        Err(AppError::Internal(
            "API Key 解密功能需要在 Infra 层实现后启用".to_string(),
        ))
    }
}