use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use crate::application::dto::provider_dto::{
    CreateProviderRequest, ProviderResponse, ProviderSummary, UpdateProviderRequest,
};
use crate::domain::log::AuditLog;
use crate::domain::provider::Provider;
use crate::domain::provider::repository::AccountRepository;
use crate::domain::log::AuditLogRepository;
use crate::domain::provider::repository::ProviderRepository;
use crate::domain::shared::EncryptionService;
use crate::domain::shared::Status;
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
            models: provider.models.clone().into(),
            status: provider.status.to_string(),
            created_at: provider.created_at.with_timezone(&chrono::Utc),
            updated_at: provider.updated_at.with_timezone(&chrono::Utc),
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
        let provider = Provider::new(
            req.name,
            req.openai_base_url,
            req.anthropic_base_url,
        )?;
        let saved = self
            .provider_repo
            .save(&provider)
            .await?;

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
            .await?
            .ok_or_else(|| AppError::NotFound(format!("提供商 {} 未找到", id)))?;

        let old_status = provider.status.to_string();

        if let Some(name) = req.name {
            provider.rename(name)?;
        }
        if let Some(url) = req.openai_base_url {
            provider.openai_base_url = Some(url).filter(|u| !u.trim().is_empty());
        }
        if let Some(url) = req.anthropic_base_url {
            provider.anthropic_base_url = Some(url).filter(|u| !u.trim().is_empty());
        }
        if let Some(models) = req.models {
            provider.set_models(models);
        }
        if let Some(status_str) = req.status {
            let status: Status = status_str
                .parse()
                .map_err(|e: AppError| AppError::Validation(e.to_string()))?;
            match status {
                Status::Enabled => provider.enable(),
                Status::Disabled => provider.disable(),
            }
        }

        let saved = self
            .provider_repo
            .save(&provider)
            .await?;

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
            .await?
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
            .await?;

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
            .await?;

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
            .await?
            .ok_or_else(|| AppError::NotFound(format!("提供商 {} 未找到", id)))?;

        self.provider_repo
            .delete(id)
            .await?;

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
    /// 自动发现失败时返回错误，让用户感知到具体原因。
    pub async fn discover_models(&self, provider_id: Uuid) -> Result<Vec<String>, AppError> {
        tracing::info!(
            "[discover_models v2] 开始为 provider {} 自动发现模型",
            provider_id
        );

        // 1. 查找 Provider
        let mut provider = self
            .provider_repo
            .find_by_id(provider_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("提供商 {} 未找到", provider_id)))?;

        // 2. 尝试自动发现 — 失败时直接抛出错误而非静默返回空
        let new_models = self.try_discover_models(&provider).await?;

        tracing::info!(
            "[discover_models v2] provider {} 自动发现得到 {} 个模型: {:?}",
            provider_id,
            new_models.len(),
            new_models
        );

        if new_models.is_empty() {
            // 上游返回了响应但模型列表为空，不覆盖已有数据，直接返回提示
            return Err(AppError::Upstream(
                "上游 /v1/models 接口返回了空模型列表，请检查 API Key 权限或 Base URL".to_string(),
            ));
        }

        // 3. 合并去重
        provider.merge_models(new_models);

        let saved = self
            .provider_repo
            .save(&provider)
            .await?;

        Ok(saved.models.into())
    }

    /// 尝试调用上游 API 发现模型
    async fn try_discover_models(&self, provider: &Provider) -> Result<Vec<String>, AppError> {
        // 1. 获取已启用的账号
        let accounts = self
            .account_repo
            .find_enabled_by_provider_id(provider.id)
            .await?;

        if accounts.is_empty() {
            tracing::warn!("[discover] provider {} 没有已启用的 Account", provider.id);
            return Err(AppError::NotFound(
                "缺少可用的 API Key，请先为此 Provider 添加 Account".to_string(),
            ));
        }

        // 2. 从第一个已启用的账号获取并解密 API Key
        let account = &accounts[0];
        tracing::info!(
            "[discover] 使用账号 {} (suffix: {})",
            account.id,
            account.api_key_suffix
        );

        let encrypted_key = self
            .account_repo
            .get_encrypted_api_key(account.id)
            .await?;

        if encrypted_key.is_empty() {
            tracing::error!(
                "[discover] 账号 {} 的 api_key_encrypted 为空! 请重新添加 API Key",
                account.id
            );
            return Err(AppError::NotFound(
                "API Key 未正确存储，请删除并重新添加 Account".to_string(),
            ));
        }

        tracing::info!(
            "[discover] 账号 {} 加密 Key 长度: {} 字节",
            account.id,
            encrypted_key.len()
        );

        let decrypted = self
            .encryption_service
            .decrypt(&encrypted_key)
            .await
            .map_err(|e| AppError::Encryption(e.to_string()))?;

        let api_key = String::from_utf8(decrypted)
            .map_err(|_| AppError::Internal("API Key 解码失败: 非法的 UTF-8 格式".to_string()))?;

        // 3. 确定基础 URL，并规范化为不含尾部 /v1 的形式
        let base_url_raw = provider
            .openai_base_url
            .as_deref()
            .or(provider.anthropic_base_url.as_deref())
            .ok_or_else(|| AppError::Internal("提供商没有配置 API 基础地址".to_string()))?;

        let url = build_models_url(base_url_raw);

        tracing::info!("[discover] 请求 URL: {}", url);

        // 4. 调用上游 /v1/models 端点
        let response = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("x-api-key", &api_key) // 兼容 Anthropic 风格
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("模型自动发现请求失败 ({}): {}", url, e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let snippet = body.chars().take(500).collect::<String>();
            return Err(AppError::Upstream(format!(
                "上游 {} 返回 {}: {}",
                url, status, snippet
            )));
        }

        // 5. 解析响应，兼容 OpenAI 与 Anthropic 两种格式
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::Upstream(format!("解析模型列表响应失败: {}", e)))?;

        // OpenAI 格式: { "data": [{ "id": "gpt-4", ... }, ...] }
        // Anthropic 格式: { "data": [{ "id": "claude-...", "type": "model" }, ...], "has_more": ... }
        // 部分代理格式: { "models": [...] } 或顶层数组
        let models = extract_model_ids(&body);

        if models.is_empty() {
            tracing::warn!(
                "上游 {} 返回了 200，但未能从响应中解析出模型 ID。响应体: {}",
                url,
                body
            );
        }

        Ok(models)
    }
}

/// 根据用户输入的 base_url 构造 `/v1/models` 完整 URL，避免 `/v1` 双拼接
fn build_models_url(base_url_raw: &str) -> String {
    let trimmed = base_url_raw.trim().trim_end_matches('/');
    // 如果用户填写的 base_url 已经包含尾部 /v1（无论大小写或多斜杠），只需拼接 /models
    if trimmed.to_lowercase().ends_with("/v1") {
        format!("{}/models", trimmed)
    } else {
        format!("{}/v1/models", trimmed)
    }
}

/// 兼容多种上游响应格式，提取模型 ID 数组
fn extract_model_ids(body: &serde_json::Value) -> Vec<String> {
    // 1. 标准 OpenAI / Anthropic: body.data[].id
    if let Some(arr) = body.get("data").and_then(|v| v.as_array()) {
        return arr
            .iter()
            .filter_map(|item| item.get("id").and_then(|v| v.as_str()).map(String::from))
            .collect();
    }
    // 2. 部分代理: body.models 数组，元素可能是字符串或 { id }
    if let Some(arr) = body.get("models").and_then(|v| v.as_array()) {
        return arr
            .iter()
            .filter_map(|item| {
                item.as_str()
                    .map(String::from)
                    .or_else(|| item.get("id").and_then(|v| v.as_str()).map(String::from))
                    .or_else(|| item.get("name").and_then(|v| v.as_str()).map(String::from))
            })
            .collect();
    }
    // 3. 顶层就是数组
    if let Some(arr) = body.as_array() {
        return arr
            .iter()
            .filter_map(|item| {
                item.as_str()
                    .map(String::from)
                    .or_else(|| item.get("id").and_then(|v| v.as_str()).map(String::from))
            })
            .collect();
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_models_url_normalizes_v1_suffix() {
        assert_eq!(
            build_models_url("https://api.openai.com/v1"),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            build_models_url("https://api.openai.com/v1/"),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            build_models_url("https://api.openai.com"),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            build_models_url("https://api.anthropic.com"),
            "https://api.anthropic.com/v1/models"
        );
    }

    #[test]
    fn extract_model_ids_handles_openai_format() {
        let body = serde_json::json!({
            "data": [
                { "id": "gpt-4", "object": "model" },
                { "id": "gpt-3.5-turbo", "object": "model" }
            ]
        });
        assert_eq!(extract_model_ids(&body), vec!["gpt-4", "gpt-3.5-turbo"]);
    }

    #[test]
    fn extract_model_ids_handles_anthropic_format() {
        let body = serde_json::json!({
            "data": [
                { "id": "claude-3-opus", "type": "model" }
            ],
            "has_more": false
        });
        assert_eq!(extract_model_ids(&body), vec!["claude-3-opus"]);
    }

    #[test]
    fn extract_model_ids_handles_models_field() {
        let body = serde_json::json!({ "models": ["m1", "m2"] });
        assert_eq!(extract_model_ids(&body), vec!["m1", "m2"]);
    }

    #[test]
    fn extract_model_ids_returns_empty_on_unknown_shape() {
        let body = serde_json::json!({ "foo": "bar" });
        assert!(extract_model_ids(&body).is_empty());
    }
}
