use std::sync::Arc;

use rand::{distributions::Alphanumeric, Rng};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::application::dto::user_dto::{CreateApiKeyResponse, UserApiKeyResponse};
use crate::domain::entities::audit_log::AuditLog;
use crate::domain::entities::user_api_key::UserApiKey;
use crate::domain::repositories::audit_log_repository::AuditLogRepository;
use crate::domain::repositories::user_api_key_repository::UserApiKeyRepository;
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
            last_used_at: key.last_used_at,
            created_at: key.created_at,
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

        let saved = self
            .api_key_repo
            .save(&entity)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 记录审计日志
        let details = serde_json::json!({
            "description": saved.description,
            "key_prefix": saved.key_prefix,
        });
        let audit = AuditLog::new(
            Some(user_id),
            "create_api_key",
            "user_api_key",
            Some(saved.id),
            Some(details),
        );
        self.audit_log_repo
            .save(&audit)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(CreateApiKeyResponse {
            id: saved.id,
            full_key,
            key_prefix: saved.key_prefix,
            description: saved.description,
            status: saved.status.to_string(),
            created_at: saved.created_at,
        })
    }

    /// 查询指定用户的所有 API key（脱敏，不返回完整 key）
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<UserApiKeyResponse>, AppError> {
        let keys = self
            .api_key_repo
            .find_all_by_user(user_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(keys.into_iter().map(|k| Self::to_response(&k)).collect())
    }

    /// 撤销指定用户的 API key（校验 key 属于当前用户）
    pub async fn revoke(&self, user_id: Uuid, key_id: Uuid) -> Result<(), AppError> {
        let key = self
            .api_key_repo
            .find_by_id(key_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("API key 未找到".to_string()))?;

        // 校验 key 属于当前用户
        if key.user_id != user_id {
            return Err(AppError::NotFound("API key 未找到".to_string()));
        }

        self.api_key_repo
            .revoke(key_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 记录审计日志
        let details = serde_json::json!({
            "key_prefix": key.key_prefix,
        });
        let audit = AuditLog::new(
            Some(user_id),
            "revoke_api_key",
            "user_api_key",
            Some(key_id),
            Some(details),
        );
        self.audit_log_repo
            .save(&audit)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 验证 API key 并返回对应的 user_id
    ///
    /// 用于代理认证场景。通过比较 SHA-256 哈希查找匹配的 key，
    /// 验证其状态为启用后返回所属用户 ID。
    pub async fn validate_api_key(&self, key: &str) -> Result<Uuid, AppError> {
        let key_hash = Self::hash_key(key);

        let api_key = self
            .api_key_repo
            .find_by_key_hash(&key_hash)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::Unauthorized("无效的 API key".to_string()))?;

        if !api_key.status.is_enabled() {
            return Err(AppError::Unauthorized("API key 已被禁用".to_string()));
        }

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
