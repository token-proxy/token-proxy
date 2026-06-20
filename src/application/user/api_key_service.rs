use std::sync::Arc;

use rand::{distributions::Alphanumeric, Rng};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::dto::{CreateApiKeyResponse, UserApiKeyResponse};
use crate::domain::log::AuditLog;
use crate::domain::log::AuditLogRepository;
use crate::domain::user::UserApiKey;
use crate::domain::user::UserApiKeyRepository;
use crate::shared::error::AppError;

/// API key 前缀标识
const API_KEY_PREFIX: &str = "tp_";

const TOKEN_RANDOM_LEN: usize = 40;

/// 用户 API key 管理服务
///
/// 负责 API key 的生成、列表查询和撤销操作。
/// 完整 key 只在创建时返回一次，后续列表查询仅返回脱敏信息。
pub struct UserApiKeyService {
    api_key_repo: Arc<dyn UserApiKeyRepository>,
    audit_log_repo: Arc<dyn AuditLogRepository>,
}

impl UserApiKeyService {
    pub fn new(
        api_key_repo: Arc<dyn UserApiKeyRepository>,
        audit_log_repo: Arc<dyn AuditLogRepository>,
    ) -> Self {
        UserApiKeyService {
            api_key_repo,
            audit_log_repo,
        }
    }

    fn to_response(key: &UserApiKey) -> UserApiKeyResponse {
        UserApiKeyResponse {
            id: key.id,
            key_prefix: key.key_prefix.clone(),
            description: key.description.clone(),
            status: key.status.to_string(),
            last_used_at: key.last_used_at_utc(),
            created_at: key.created_at_utc(),
        }
    }

    /// SHA-256 哈希
    fn hash_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn generate_key() -> String {
        let random: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(TOKEN_RANDOM_LEN)
            .map(char::from)
            .collect();
        format!("{}{}", API_KEY_PREFIX, random)
    }

    /// 为指定用户创建新的 API key（完整 key 仅在创建时返回）
    pub async fn create(
        &self,
        user_id: Uuid,
        description: String,
    ) -> Result<CreateApiKeyResponse, AppError> {
        let trimmed = description.trim().to_string();
        if trimmed.is_empty() {
            return Err(AppError::Validation("描述不能为空".to_string()));
        }

        let full_key = Self::generate_key();
        let key_hash = Self::hash_key(&full_key);

        // 计算 prefix（显示前 12 位 + `...`）
        let prefix_len = std::cmp::min(full_key.len(), 12);
        let key_prefix = format!("{}...", &full_key[..prefix_len]);

        let entity = UserApiKey::new(user_id, key_hash, key_prefix, trimmed);

        let saved = self.api_key_repo.save(&entity).await?;

        // 记录审计日志
        let details = serde_json::json!({
            "description": saved.description,
            "key_prefix": saved.key_prefix,
        });
        let audit = AuditLog::new(
            Some(user_id),
            "user",
            "create_api_key",
            "user_api_key",
            Some(saved.id),
            Some(details),
        );
        self.audit_log_repo.save(&audit).await?;

        Ok(CreateApiKeyResponse {
            id: saved.id,
            full_key,
            key_prefix: saved.key_prefix.clone(),
            description: saved.description.clone(),
            status: saved.status.to_string(),
            created_at: saved.created_at_utc(),
        })
    }

    /// 查询指定用户的所有 API key（脱敏，不返回完整 key）
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<UserApiKeyResponse>, AppError> {
        let keys = self.api_key_repo.find_all_by_user(user_id).await?;

        Ok(keys.into_iter().map(|k| Self::to_response(&k)).collect())
    }

    /// 撤销指定用户的 API key（校验 key 属于当前用户）
    pub async fn revoke(&self, user_id: Uuid, key_id: Uuid) -> Result<(), AppError> {
        let key = self
            .api_key_repo
            .find_by_id(key_id)
            .await?
            .ok_or_else(|| AppError::NotFound("API key 未找到".to_string()))?;

        // 校验 key 属于当前用户
        if key.user_id != user_id {
            return Err(AppError::NotFound("API key 未找到".to_string()));
        }

        self.revoke_key_and_audit(key_id, &key, Some(user_id)).await
    }

    /// 管理员吊销任意用户的 API key（跳过所有权校验）
    pub async fn admin_revoke(&self, key_id: Uuid) -> Result<(), AppError> {
        let key = self
            .api_key_repo
            .find_by_id(key_id)
            .await?
            .ok_or_else(|| AppError::NotFound("API key 未找到".to_string()))?;

        self.revoke_key_and_audit(key_id, &key, None).await
    }

    /// 执行吊销操作并写入审计日志
    async fn revoke_key_and_audit(
        &self,
        key_id: Uuid,
        key: &UserApiKey,
        operator_user_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        self.api_key_repo.revoke(key_id).await?;

        let details = serde_json::json!({
            "key_prefix": key.key_prefix,
        });
        let audit = AuditLog::new(
            operator_user_id,
            "user",
            "revoke_api_key",
            "user_api_key",
            Some(key_id),
            Some(details),
        );
        self.audit_log_repo.save(&audit).await?;

        Ok(())
    }

    /// 更新 API key 备注（校验 key 属于当前用户）
    pub async fn update_description(
        &self,
        user_id: Uuid,
        key_id: Uuid,
        description: String,
    ) -> Result<UserApiKeyResponse, AppError> {
        let trimmed = description.trim().to_string();
        if trimmed.is_empty() {
            return Err(AppError::Validation("描述不能为空".to_string()));
        }

        let key = self
            .api_key_repo
            .find_by_id(key_id)
            .await?
            .ok_or_else(|| AppError::NotFound("API key 未找到".to_string()))?;

        if key.user_id != user_id {
            return Err(AppError::NotFound("API key 未找到".to_string()));
        }

        let mut updated = key;
        updated.description = trimmed;

        let saved = self.api_key_repo.save(&updated).await?;

        Ok(Self::to_response(&saved))
    }

    /// 验证 API key 并返回对应的 user_id
    ///
    /// 用于代理认证场景。通过比较 SHA-256 哈希查找匹配的 key，
    /// 验证其状态为启用后返回所属用户 ID，并更新 last_used_at。
    pub async fn validate_api_key(&self, key: &str) -> Result<Uuid, AppError> {
        let key_hash = Self::hash_key(key);

        let api_key = self
            .api_key_repo
            .find_by_key_hash(&key_hash)
            .await?
            .ok_or_else(|| AppError::Unauthorized("无效的 API key".to_string()))?;

        if !api_key.status.is_enabled() {
            return Err(AppError::Unauthorized("API key 已被禁用".to_string()));
        }

        // 更新最后使用时间（忽略失败，不阻塞请求）
        self.api_key_repo.update_last_used(api_key.id).await.ok();

        Ok(api_key.user_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key_format() {
        let key = UserApiKeyService::generate_key();
        assert!(key.starts_with("tp_"));
        assert!(key.len() > 10);
    }

    #[test]
    fn test_key_hash_is_deterministic() {
        let key = "tp_test_key_123";
        let hash1 = UserApiKeyService::hash_key(key);
        let hash2 = UserApiKeyService::hash_key(key);
        assert_eq!(hash1, hash2);
    }
}
