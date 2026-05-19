use std::sync::Arc;

use uuid::Uuid;

use crate::application::dto::provider_dto::{
    CreateProviderRequest, ProviderResponse, ProviderSummary, UpdateProviderRequest,
};
use crate::domain::entities::provider::Provider;
use crate::domain::repositories::account_repository::AccountRepository;
use crate::domain::repositories::provider_repository::ProviderRepository;
use crate::domain::value_objects::status::Status;
use crate::shared::error::AppError;

pub struct ProviderService {
    provider_repo: Arc<dyn ProviderRepository>,
    account_repo: Arc<dyn AccountRepository>,
}

impl ProviderService {
    pub fn new(
        provider_repo: Arc<dyn ProviderRepository>,
        account_repo: Arc<dyn AccountRepository>,
    ) -> Self {
        ProviderService {
            provider_repo,
            account_repo,
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

    pub async fn create(&self, req: CreateProviderRequest) -> Result<ProviderResponse, AppError> {
        let provider = Provider::new(req.name, req.openai_base_url, req.anthropic_base_url)?;
        let saved = self
            .provider_repo
            .save(&provider)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(Self::to_response(&saved, Some(0)))
    }

    pub async fn update(
        &self,
        id: Uuid,
        req: UpdateProviderRequest,
    ) -> Result<ProviderResponse, AppError> {
        let mut provider = self
            .provider_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("提供商 {} 未找到", id)))?;

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

    pub async fn delete(&self, id: Uuid) -> Result<(), AppError> {
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

        self.provider_repo
            .delete(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}