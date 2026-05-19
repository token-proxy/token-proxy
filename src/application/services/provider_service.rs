use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use crate::application::dto::provider_dto::{
    CreateProviderRequest, ProviderResponse, ProviderSummary, UpdateProviderRequest,
};
use crate::domain::entities::audit_log::AuditLog;
use crate::domain::entities::provider::Provider;
use crate::domain::repositories::account_repository::AccountRepository;
use crate::domain::repositories::audit_log_repository::AuditLogRepository;
use crate::domain::repositories::provider_repository::ProviderRepository;
use crate::domain::services::encryption_service::EncryptionService;
use crate::domain::value_objects::status::Status;
use crate::shared::error::AppError;

pub struct ProviderService {
    provider_repo: Arc<dyn ProviderRepository>,
    account_repo: Arc<dyn AccountRepository>,
    audit_log_repo: Arc<dyn AuditLogRepository>,
    encryption_service: Arc<dyn EncryptionService>,
    http_client: reqwest::Client,
}

impl ProviderService {
    pub fn new(
        provider_repo: Arc<dyn ProviderRepository>,
        account_repo: Arc<dyn AccountRepository>,
        audit_log_repo: Arc<dyn AuditLogRepository>,
        encryption_service: Arc<dyn EncryptionService>,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("创建 HTTP 客户端失败");

        ProviderService {
            provider_repo,
            account_repo,
            audit_log_repo,
            encryption_service,
            http_client,
        }
    }

    fn to_response(provider: &Provider, account_count: Option<i64>) -> ProviderResponse {
        ProviderResponse {
            id: provider.id,
            name: provider.name.clone(),
            openai_base_url: provider.openai_base_url.clone(),
            anthropic_base_url: provider.anthropic_base_url.clone(),
            models: provider.models.clone(),
            status: provider.status.to_string(),
            created_at: provider.created_at,
            updated_at: provider.updated_at,
            account_count,
        }
    }

    #[allow(dead_code)]
    fn to_summary(provider: &Provider, account_count: i64) -> ProviderSummary {
        ProviderSummary {
            id: provider.id,
            name: provider.name.clone(),
            status: provider.status.to_string(),
            account_count,
        }
    }

    pub async fn create(
        &self,
        req: CreateProviderRequest,
        user_id: Option<Uuid>,
    ) -> Result<ProviderResponse, AppError> {
        let provider = Provider::new(req.name, req.openai_base_url, req.anthropic_base_url)?;
        let saved = self
            .provider_repo
            .save(&provider)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 记录审计日志
        self.log_audit(
            user_id,
            "create",
            "provider",
            Some(provider.id),
            Some(serde_json::json!({
                "name": &provider.name,
                "openai_base_url": &provider.openai_base_url,
                "anthropic_base_url": &provider.anthropic_base_url,
            })),
        )
        .await;

        Ok(Self::to_response(&saved, Some(0)))
    }

    pub async fn update(
        &self,
        id: Uuid,
        req: UpdateProviderRequest,
        user_id: Option<Uuid>,
    ) -> Result<ProviderResponse, AppError> {
        let mut provider = self
            .provider_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("提供商 {} 未找到", id)))?;

        let old_status = provider.status.to_string();

        if let Some(name) = req.name {
            let trimmed = name.trim().to_string();
            if trimmed.is_empty() {
                return Err(AppError::Validation("提供商名称不能为空".to_string()));
            }
            provider.name = trimmed;
        }
        if let Some(url) = req.openai_base_url {
            provider.openai_base_url = Some(url).filter(|u| !u.trim().is_empty());
        }
        if let Some(url) = req.anthropic_base_url {
            provider.anthropic_base_url = Some(url).filter(|u| !u.trim().is_empty());
        }
        if let Some(models) = req.models {
            provider.models = models;
        }
        if let Some(status_str) = req.status {
            let status: Status = status_str
                .parse()
                .map_err(|e: AppError| AppError::Validation(e.to_string()))?;
            provider.status = status;
        }
        provider.updated_at = chrono::Utc::now();

        let saved = self
            .provider_repo
            .save(&provider)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let account_count = self
            .account_repo
            .find_by_provider_id(id)
            .await
            .map(|accounts| accounts.len() as i64)
            .unwrap_or(0);

        // 记录审计日志
        let new_status = provider.status.to_string();
        let audit_action = if old_status != new_status {
            if new_status == "enabled" {
                "enable"
            } else {
                "disable"
            }
        } else {
            "update"
        };

        self.log_audit(
            user_id,
            audit_action,
            "provider",
            Some(id),
            Some(serde_json::json!({
                "name": &provider.name,
                "old_status": old_status,
                "new_status": new_status,
            })),
        )
        .await;

        Ok(Self::to_response(&saved, Some(account_count)))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<ProviderResponse, AppError> {
        let provider = self
            .provider_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("提供商 {} 未找到", id)))?;

        let account_count = self
            .account_repo
            .find_by_provider_id(id)
            .await
            .map(|accounts| accounts.len() as i64)
            .unwrap_or(0);

        Ok(Self::to_response(&provider, Some(account_count)))
    }

    pub async fn list_all(&self) -> Result<Vec<ProviderResponse>, AppError> {
        let providers = self
            .provider_repo
            .find_all()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut results = Vec::with_capacity(providers.len());
        for provider in &providers {
            let account_count = self
                .account_repo
                .find_by_provider_id(provider.id)
                .await
                .map(|accounts| accounts.len() as i64)
                .unwrap_or(0);
            results.push(Self::to_response(provider, Some(account_count)));
        }

        Ok(results)
    }

    pub async fn delete(&self, id: Uuid, user_id: Option<Uuid>) -> Result<(), AppError> {
        let accounts = self
            .account_repo
            .find_by_provider_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if !accounts.is_empty() {
            return Err(AppError::Conflict(format!(
                "提供商 {} 下存在 {} 个关联账号，请先删除账号",
                id,
                accounts.len()
            )));
        }

        let provider = self
            .provider_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("提供商 {} 未找到", id)))?;

        self.provider_repo
            .delete(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 记录审计日志
        self.log_audit(
            user_id,
            "delete",
            "provider",
            Some(id),
            Some(serde_json::json!({
                "name": provider.name,
            })),
        )
        .await;

        Ok(())
    }

    /// 写入审计日志（异步，忽略错误）
    async fn log_audit(
        &self,
        user_id: Option<Uuid>,
        action: &str,
        entity_type: &str,
        entity_id: Option<Uuid>,
        details: Option<serde_json::Value>,
    ) {
        let log = AuditLog::new(user_id, action, entity_type, entity_id, details);
        if let Err(e) = self.audit_log_repo.save(&log).await {
            tracing::error!("审计日志写入失败: {}", e);
        }
    }

    /// 自动发现模型
    ///
    /// 调用上游 `/v1/models` 端点获取模型列表，合并去重后更新 Provider。
    /// 如果自动发现失败，返回当前已有的模型列表，不返回错误。
    pub async fn discover_models(&self, provider_id: Uuid) -> Result<Vec<String>, AppError> {
        // 1. 查找 Provider
        let mut provider = self
            .provider_repo
            .find_by_id(provider_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("提供商 {} 未找到", provider_id)))?;

        // 2. 尝试自动发现，失败时返回已有模型
        let discovered = self.try_discover_models(&provider).await;
        let new_models = match discovered {
            Ok(models) => models,
            Err(e) => {
                tracing::warn!(
                    "提供商 {} 模型自动发现失败: {}，返回已有模型列表",
                    provider_id,
                    e
                );
                return Ok(provider.models.clone());
            }
        };

        // 3. 合并去重
        provider.models.extend(new_models);
        provider.models.sort();
        provider.models.dedup();
        provider.updated_at = chrono::Utc::now();

        let saved = self
            .provider_repo
            .save(&provider)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(saved.models)
    }

    /// 尝试调用上游 API 发现模型
    async fn try_discover_models(&self, provider: &Provider) -> Result<Vec<String>, AppError> {
        // 1. 获取已启用的账号
        let accounts = self
            .account_repo
            .find_enabled_by_provider_id(provider.id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if accounts.is_empty() {
            return Err(AppError::NotFound("无可用账号".to_string()));
        }

        // 2. 从第一个已启用的账号获取并解密 API Key
        let account = &accounts[0];
        let encrypted_key = self
            .account_repo
            .get_encrypted_api_key(account.id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if encrypted_key.is_empty() {
            return Err(AppError::NotFound(
                "账号未配置有效的 API Key".to_string(),
            ));
        }

        let decrypted = self
            .encryption_service
            .decrypt(&encrypted_key)
            .await
            .map_err(|e| AppError::Encryption(e.to_string()))?;

        let api_key = String::from_utf8(decrypted)
            .map_err(|_| AppError::Internal("API Key 解码失败: 非法的 UTF-8 格式".to_string()))?;

        // 3. 确定基础 URL
        let base_url = provider
            .openai_base_url
            .as_deref()
            .or(provider.anthropic_base_url.as_deref())
            .ok_or_else(|| AppError::Internal("提供商没有配置 API 基础地址".to_string()))?;

        let url = format!("{}/v1/models", base_url.trim_end_matches('/'));

        // 4. 调用上游 /v1/models 端点
        let response = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| {
                AppError::Upstream(format!("模型自动发现请求失败: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Upstream(format!(
                "模型自动发现请求返回错误状态 {}: {}",
                status, body
            )));
        }

        // 5. 解析响应，提取模型 ID 列表
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| {
                AppError::Upstream(format!("解析模型列表响应失败: {}", e))
            })?;

        let models = body["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item["id"].as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        Ok(models)
    }
}