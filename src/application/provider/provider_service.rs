//! 服务商应用服务 — application/provider/
//!
//! 编排 Provider 的 CRUD、模型自动发现操作。
//! 包含审计日志写入和上游 API 调用能力。

use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use super::dto::{CreateProviderRequest, ProviderResponse, ProviderSummary, UpdateProviderRequest};
use crate::domain::log::AuditAction;
use crate::domain::log::AuditEntityType;
use crate::domain::log::AuditLog;
use crate::domain::log::AuditLogRepository;
use crate::domain::provider::repository::AccountRepository;
use crate::domain::provider::repository::ProviderRepository;
use crate::domain::provider::Provider;
use crate::domain::shared::EncryptionService;
use crate::domain::shared::Status;
use crate::shared::error::AppError;

/// 服务商应用服务
///
/// 编排服务商 CRUD、模型自动发现、审计日志记录。
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

    fn to_response(
        provider: &Provider,
        account_count: Option<i64>,
        available_account_count: Option<i64>,
    ) -> ProviderResponse {
        ProviderResponse {
            id: provider.id,
            name: provider.name.clone(),
            openai_base_url: provider.openai_base_url.clone(),
            anthropic_base_url: provider.anthropic_base_url.clone(),
            models: provider.models.clone().into(),
            rate_limit_config: provider.rate_limit_config.clone(),
            balance_exhausted_config: provider.balance_exhausted_config.clone(),
            status: provider.status.to_string(),
            created_at: provider.created_at.with_timezone(&chrono::Utc),
            updated_at: provider.updated_at.with_timezone(&chrono::Utc),
            account_count,
            available_account_count,
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

    /// 创建服务商
    ///
    /// 保存 Provider 并记录创建审计日志。
    pub async fn create(
        &self,
        req: CreateProviderRequest,
        user_id: Option<Uuid>,
    ) -> Result<ProviderResponse, AppError> {
        let mut provider = Provider::new(req.name, req.openai_base_url, req.anthropic_base_url)?;
        provider.rate_limit_config = req.rate_limit_config;
        provider.balance_exhausted_config = req.balance_exhausted_config;
        let saved = self.provider_repo.save(&provider).await?;

        // 记录审计日志
        self.log_audit(
            user_id,
            AuditAction::Create,
            AuditEntityType::Provider,
            Some(provider.id),
            Some(serde_json::json!({
                "name": &provider.name,
                "openai_base_url": &provider.openai_base_url,
                "anthropic_base_url": &provider.anthropic_base_url,
            })),
        )
        .await;

        Ok(Self::to_response(&saved, Some(0), Some(0)))
    }

    /// 更新服务商
    ///
    /// 支持更新名称、地址、模型列表、故障配置和状态。
    /// 状态变更时自动触发 enable/disable 领域行为。
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
            .ok_or_else(|| AppError::NotFound(format!("服务商 {} 未找到", id)))?;

        let old_status = provider.status.to_string();

        if let Some(name) = req.name {
            provider.rename(name)?;
        }
        if let Some(url) = req.openai_base_url {
            provider.set_openai_base_url(Some(url));
        }
        if let Some(url) = req.anthropic_base_url {
            provider.set_anthropic_base_url(Some(url));
        }
        if let Some(models) = req.models {
            provider.set_models(models);
        }
        if req.rate_limit_config.is_some() {
            provider.rate_limit_config = req.rate_limit_config;
        }
        if req.balance_exhausted_config.is_some() {
            provider.balance_exhausted_config = req.balance_exhausted_config;
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

        let saved = self.provider_repo.save(&provider).await?;

        let accounts = self
            .account_repo
            .find_by_provider_id(id)
            .await
            .unwrap_or_default();
        let account_count = accounts.len() as i64;
        let available_account_count =
            accounts.iter().filter(|a| a.status.is_enabled()).count() as i64;

        // 记录审计日志
        let new_status = provider.status.to_string();
        let audit_action = if old_status != new_status {
            if new_status == "enabled" {
                AuditAction::Enable
            } else {
                AuditAction::Disable
            }
        } else {
            AuditAction::Update
        };

        self.log_audit(
            user_id,
            audit_action,
            AuditEntityType::Provider,
            Some(id),
            Some(serde_json::json!({
                "name": &provider.name,
                "old_status": old_status,
                "new_status": new_status,
            })),
        )
        .await;

        Ok(Self::to_response(
            &saved,
            Some(account_count),
            Some(available_account_count),
        ))
    }

    /// 根据 ID 查询服务商（含账号统计）
    pub async fn get_by_id(&self, id: Uuid) -> Result<ProviderResponse, AppError> {
        let provider = self
            .provider_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("服务商 {} 未找到", id)))?;

        let accounts = self
            .account_repo
            .find_by_provider_id(id)
            .await
            .unwrap_or_default();
        let account_count = accounts.len() as i64;
        let available_account_count =
            accounts.iter().filter(|a| a.status.is_enabled()).count() as i64;

        Ok(Self::to_response(
            &provider,
            Some(account_count),
            Some(available_account_count),
        ))
    }

    /// 查询所有服务商列表（含账号统计）
    pub async fn list_all(&self) -> Result<Vec<ProviderResponse>, AppError> {
        let providers = self.provider_repo.find_all().await?;

        let mut results = Vec::with_capacity(providers.len());
        for provider in &providers {
            let accounts = self
                .account_repo
                .find_by_provider_id(provider.id)
                .await
                .unwrap_or_default();
            let account_count = accounts.len() as i64;
            let available_account_count =
                accounts.iter().filter(|a| a.status.is_enabled()).count() as i64;
            results.push(Self::to_response(
                provider,
                Some(account_count),
                Some(available_account_count),
            ));
        }

        Ok(results)
    }

    /// 删除服务商（需先清空关联账号）
    pub async fn delete(&self, id: Uuid, user_id: Option<Uuid>) -> Result<(), AppError> {
        let accounts = self.account_repo.find_by_provider_id(id).await?;

        if !accounts.is_empty() {
            return Err(AppError::Conflict(format!(
                "服务商 {} 下存在 {} 个关联账号，请先删除账号",
                id,
                accounts.len()
            )));
        }

        let provider = self
            .provider_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("服务商 {} 未找到", id)))?;

        self.provider_repo.delete(id).await?;

        // 记录审计日志
        self.log_audit(
            user_id,
            AuditAction::Delete,
            AuditEntityType::Provider,
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
        operator_id: Option<Uuid>,
        action: AuditAction,
        entity_type: AuditEntityType,
        entity_id: Option<Uuid>,
        details: Option<serde_json::Value>,
    ) {
        let log = AuditLog::new(operator_id, "user", action, entity_type, entity_id, details);
        if let Err(e) = self.audit_log_repo.save(&log).await {
            tracing::error!(error = %e, "审计日志写入失败");
        }
    }

    /// 自动发现模型
    ///
    /// 调用上游 `/v1/models` 端点获取模型列表，合并去重后更新 Provider。
    /// 自动发现失败时返回错误，让用户感知到具体原因。
    pub async fn discover_models(
        &self,
        provider_id: Uuid,
        operator_id: Option<Uuid>,
    ) -> Result<Vec<String>, AppError> {
        tracing::info!(
            provider_id = %provider_id,
            "开始自动发现模型",
        );

        // 1. 查找 Provider
        let mut provider = self
            .provider_repo
            .find_by_id(provider_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("服务商 {} 未找到", provider_id)))?;

        // 2. 尝试自动发现 — 失败时直接抛出错误而非静默返回空
        let new_models = self.try_discover_models(&provider).await?;

        tracing::info!(
            provider_id = %provider_id,
            count = new_models.len(),
            models = ?new_models,
            "模型自动发现完成",
        );

        if new_models.is_empty() {
            // 上游返回了响应但模型列表为空，不覆盖已有数据，直接返回提示
            return Err(AppError::Upstream(
                "上游 /v1/models 接口返回了空模型列表，请检查 API Key 权限或 Base URL".to_string(),
            ));
        }

        // 3. 合并去重
        provider.merge_models(new_models);

        let saved = self.provider_repo.save(&provider).await?;
        let new_models_count = saved.models.len() as u64;

        // 记录审计日志
        self.log_audit(
            operator_id,
            AuditAction::DiscoverModels,
            AuditEntityType::Provider,
            Some(provider_id),
            Some(serde_json::json!({"models_count": new_models_count})),
        )
        .await;

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
            tracing::warn!(provider_id = %provider.id, "Provider 没有已启用的 Account");
            return Err(AppError::NotFound(
                "缺少可用的 API Key，请先为此 Provider 添加 Account".to_string(),
            ));
        }

        // 2. 从第一个已启用的账号获取并解密 API Key
        let account = &accounts[0];
        tracing::info!(
            account_id = %account.id,
            suffix = %account.api_key_suffix,
            "使用账号自动发现模型",
        );

        let encrypted_key = self.account_repo.get_encrypted_api_key(account.id).await?;

        if encrypted_key.is_empty() {
            tracing::error!(
                account_id = %account.id,
                "账号加密 Key 为空，请重新添加 API Key",
            );
            return Err(AppError::NotFound(
                "API Key 未正确存储，请删除并重新添加 Account".to_string(),
            ));
        }

        tracing::info!(
            account_id = %account.id,
            key_len = encrypted_key.len(),
            "加密 Key 长度",
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
            .ok_or_else(|| AppError::Internal("服务商没有配置 API 基础地址".to_string()))?;

        let url = build_models_url(base_url_raw);

        tracing::info!(url = %url, "模型发现请求 URL");

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
                url = %url,
                response_body = %body.to_string().chars().take(300).collect::<String>(),
                "上游返回 200 但未能解析出模型 ID",
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
