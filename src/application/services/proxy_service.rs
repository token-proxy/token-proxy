use std::sync::Arc;

use axum::http::HeaderMap;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::domain::entities::access_point::AccessPoint;
use crate::domain::entities::account::Account;
use crate::domain::entities::provider::Provider;
use crate::domain::repositories::access_point_repository::AccessPointRepository;
use crate::domain::repositories::account_repository::AccountRepository;
use crate::domain::repositories::provider_repository::ProviderRepository;
use crate::domain::repositories::user_api_key_repository::UserApiKeyRepository;
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
    encryption_service: Arc<dyn EncryptionService>,
    user_api_key_repo: Arc<dyn UserApiKeyRepository>,
}

impl ProxyService {
    pub fn new(
        access_point_repo: Arc<dyn AccessPointRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        account_repo: Arc<dyn AccountRepository>,
        encryption_service: Arc<dyn EncryptionService>,
        user_api_key_repo: Arc<dyn UserApiKeyRepository>,
    ) -> Self {
        ProxyService {
            access_point_repo,
            provider_repo,
            account_repo,
            encryption_service,
            user_api_key_repo,
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
                AppError::NotFound(format!("接入点 '{}' 关联的提供商未找到", short_code))
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
                AppError::NotFound(format!("接入点 '{}' 关联的账号未找到", short_code))
            })?;

        if !account.status.is_enabled() {
            return Err(AppError::Forbidden(format!(
                "接入点 '{}' 关联的账号已被禁用",
                short_code
            )));
        }

        // 4. 解密 API Key
        let decrypted_api_key = self.decrypt_account_key(account.id).await?;

        Ok(ProxyContext {
            access_point,
            provider,
            account,
            decrypted_api_key,
        })
    }

    /// 解密账号的 API Key
    async fn decrypt_account_key(&self, account_id: Uuid) -> Result<String, AppError> {
        let encrypted_key = self
            .account_repo
            .get_encrypted_api_key(account_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if encrypted_key.is_empty() {
            return Err(AppError::NotFound(
                "API Key 未正确存储，请删除并重新添加 Account".to_string(),
            ));
        }

        let decrypted = self
            .encryption_service
            .decrypt(&encrypted_key)
            .await
            .map_err(|e| AppError::Encryption(e.to_string()))?;

        String::from_utf8(decrypted)
            .map_err(|_| AppError::Internal("API Key 解码失败: 非法的 UTF-8 格式".to_string()))
    }

    /// 认证用户 API key
    ///
    /// 从请求头中提取 `Authorization: Bearer <token>`，计算 SHA-256 hex，
    /// 在 UserApiKeyRepository 中查找已启用的 key，认证通过后更新 last_used_at。
    ///
    /// # 返回
    /// - `Ok(user_id)` — 认证成功，返回 key 所属用户 ID
    /// - `Err(AppError::Unauthorized)` — 认证失败
    pub async fn authenticate_request(&self, headers: &HeaderMap) -> Result<Uuid, AppError> {
        // 1. 提取 Authorization header
        let auth_header = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("缺少 Authorization 请求头".to_string()))?;

        // 2. 提取 Bearer token
        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            AppError::Unauthorized("Authorization 格式应为 Bearer <token>".to_string())
        })?;

        if token.is_empty() {
            return Err(AppError::Unauthorized("API key 不能为空".to_string()));
        }

        // 3. 计算 SHA-256 hex
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let key_hash = format!("{:x}", hasher.finalize());

        // 4. 查找 key
        let api_key = self
            .user_api_key_repo
            .find_by_key_hash(&key_hash)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::Unauthorized("API key 无效或已被撤销".to_string()))?;

        // 5. 检查状态
        if !api_key.status.is_enabled() {
            return Err(AppError::Unauthorized("API key 已被禁用".to_string()));
        }

        // 6. 更新最后使用时间（忽略失败，不阻塞请求）
        self.user_api_key_repo
            .update_last_used(api_key.id)
            .await
            .ok();

        Ok(api_key.user_id)
    }
}
