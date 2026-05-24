use std::sync::Arc;

use uuid::Uuid;

use crate::application::dto::access_point_dto::{
    AccessPointResponse, CreateAccessPointRequest, ModelMappingDto, UpdateAccessPointRequest,
};
use crate::domain::entities::access_point::AccessPoint;
use crate::domain::repositories::access_point_repository::AccessPointRepository;
use crate::domain::repositories::account_repository::AccountRepository;
use crate::domain::repositories::provider_repository::ProviderRepository;
use crate::domain::value_objects::access_point_type::AccessPointType;
use crate::domain::value_objects::model_mapping::ModelMapping;
use crate::domain::value_objects::short_code::ShortCode;
use crate::domain::value_objects::status::Status;
use crate::shared::error::AppError;

pub struct AccessPointService {
    access_point_repo: Arc<dyn AccessPointRepository>,
    provider_repo: Arc<dyn ProviderRepository>,
    account_repo: Arc<dyn AccountRepository>,
}

impl AccessPointService {
    pub fn new(
        access_point_repo: Arc<dyn AccessPointRepository>,
        provider_repo: Arc<dyn ProviderRepository>,
        account_repo: Arc<dyn AccountRepository>,
    ) -> Self {
        AccessPointService {
            access_point_repo,
            provider_repo,
            account_repo,
        }
    }

    fn to_response(ap: &AccessPoint) -> AccessPointResponse {
        let mappings: Vec<ModelMappingDto> = ap
            .model_mappings
            .iter()
            .map(|m| ModelMappingDto {
                source_model: m.source_model.clone(),
                target_model: m.target_model.clone(),
            })
            .collect();

        AccessPointResponse {
            id: ap.id,
            name: ap.name.clone(),
            api_type: ap.api_type.to_string(),
            short_code: ap.short_code.to_string(),
            provider_id: ap.provider_id,
            account_id: ap.account_id,
            model_mappings: mappings,
            access_url: format!("/ap/{}", ap.short_code),
            status: ap.status.to_string(),
            created_at: ap.created_at,
            updated_at: ap.updated_at,
        }
    }

    pub async fn create(
        &self,
        req: CreateAccessPointRequest,
        created_by: Uuid,
    ) -> Result<AccessPointResponse, AppError> {
        // 检查短码唯一性（如果提供了短码）
        let short_code = if let Some(ref code_str) = req.short_code {
            let code = ShortCode::new(code_str)?;
            let existing = self
                .access_point_repo
                .find_by_short_code(code.as_str())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
            if existing.is_some() {
                return Err(AppError::Conflict(format!("短码 '{}' 已被使用", code)));
            }
            code
        } else {
            // 自动生成，并确保唯一性
            loop {
                let code = ShortCode::generate();
                let existing = self
                    .access_point_repo
                    .find_by_short_code(code.as_str())
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;
                if existing.is_none() {
                    break code;
                }
            }
        };

        // 检查 Provider 存在性
        let provider = self
            .provider_repo
            .find_by_id(req.provider_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("提供商 {} 未找到", req.provider_id)))?;

        if !provider.status.is_enabled() {
            return Err(AppError::Conflict("关联的提供商已被禁用".to_string()));
        }

        // 检查 Account 存在性
        let account = self
            .account_repo
            .find_by_id(req.account_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("账号 {} 未找到", req.account_id)))?;

        if !account.status.is_enabled() {
            return Err(AppError::Conflict("关联的账号已被禁用".to_string()));
        }

        // 默认接入类型为 Anthropic
        let api_type = AccessPointType::Anthropic;

        let mut access_point = AccessPoint::new(
            req.name,
            api_type,
            short_code,
            req.provider_id,
            req.account_id,
            created_by,
        );

        // 处理模型映射
        if let Some(mappings) = req.model_mappings {
            access_point.model_mappings = mappings
                .into_iter()
                .map(|m| ModelMapping {
                    source_model: m.source_model,
                    target_model: m.target_model,
                })
                .collect();
        }

        let saved = self
            .access_point_repo
            .save(&access_point)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Self::to_response(&saved))
    }

    pub async fn update(
        &self,
        id: Uuid,
        req: UpdateAccessPointRequest,
    ) -> Result<AccessPointResponse, AppError> {
        let mut ap = self
            .access_point_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("接入点 {} 未找到", id)))?;

        if let Some(name) = req.name {
            let trimmed = name.trim().to_string();
            if trimmed.is_empty() {
                return Err(AppError::Validation("接入点名称不能为空".to_string()));
            }
            ap.name = trimmed;
        }

        if let Some(provider_id) = req.provider_id {
            let provider = self
                .provider_repo
                .find_by_id(provider_id)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?
                .ok_or_else(|| AppError::NotFound(format!("提供商 {} 未找到", provider_id)))?;
            if !provider.status.is_enabled() {
                return Err(AppError::Conflict("关联的提供商已被禁用".to_string()));
            }
            ap.provider_id = provider_id;
        }

        if let Some(account_id) = req.account_id {
            let account = self
                .account_repo
                .find_by_id(account_id)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?
                .ok_or_else(|| AppError::NotFound(format!("账号 {} 未找到", account_id)))?;
            if !account.status.is_enabled() {
                return Err(AppError::Conflict("关联的账号已被禁用".to_string()));
            }
            ap.account_id = account_id;
        }

        if let Some(mappings) = req.model_mappings {
            ap.model_mappings = mappings
                .into_iter()
                .map(|m| ModelMapping {
                    source_model: m.source_model,
                    target_model: m.target_model,
                })
                .collect();
        }

        if let Some(status_str) = req.status {
            let status: Status = status_str
                .parse()
                .map_err(|e: AppError| AppError::Validation(e.to_string()))?;
            ap.status = status;
        }

        ap.updated_at = chrono::Utc::now();

        let saved = self
            .access_point_repo
            .save(&ap)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Self::to_response(&saved))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<AccessPointResponse, AppError> {
        let ap = self
            .access_point_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("接入点 {} 未找到", id)))?;

        Ok(Self::to_response(&ap))
    }

    pub async fn get_by_short_code(
        &self,
        short_code: &str,
    ) -> Result<AccessPointResponse, AppError> {
        let ap = self
            .access_point_repo
            .find_by_short_code(short_code)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("接入点 {} 未找到", short_code)))?;

        Ok(Self::to_response(&ap))
    }

    pub async fn list_all(&self) -> Result<Vec<AccessPointResponse>, AppError> {
        let points = self
            .access_point_repo
            .find_all()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(points.iter().map(Self::to_response).collect())
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        self.access_point_repo
            .delete(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}
