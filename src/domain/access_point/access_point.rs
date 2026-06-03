use sea_orm::entity::prelude::*;

use crate::domain::access_point::model_mapping::ModelMappingCollection;
use crate::domain::access_point::short_code::ShortCode;
use crate::domain::shared::status::Status;
use crate::domain::shared::AccessPointType;
use crate::domain::shared::EncryptionService;
use crate::shared::error::AppError;
use chrono::{DateTime, FixedOffset, Utc};
use uuid::Uuid;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "access_points")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub api_type: AccessPointType,
    #[sea_orm(unique)]
    pub short_code: ShortCode,
    pub provider_id: Uuid,
    pub account_id: Uuid,
    pub model_mappings: ModelMappingCollection,
    pub status: Status,
    pub created_by: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,

    #[sea_orm(belongs_to, from = "provider_id", to = "id")]
    pub provider: HasOne<super::super::provider::provider::Entity>,

    #[sea_orm(belongs_to, from = "account_id", to = "id")]
    pub account: HasOne<super::super::provider::account::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

// ─── Model 基础行为 ────────────────────────────────────────────────

impl Model {
    pub fn new(
        name: String,
        api_type: AccessPointType,
        short_code: ShortCode,
        provider_id: Uuid,
        account_id: Uuid,
        created_by: Uuid,
    ) -> Self {
        let now = Utc::now();
        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        Model {
            id: Uuid::new_v4(),
            name,
            api_type,
            short_code,
            provider_id,
            account_id,
            model_mappings: ModelMappingCollection::default(),
            status: Status::Enabled,
            created_by,
            created_at: now.with_timezone(&offset),
            updated_at: now.with_timezone(&offset),
        }
    }

    pub fn resolve_model(
        &self,
        requested_model: &str,
        default_model: Option<&str>,
    ) -> String {
        let mapped = if requested_model.is_empty() {
            None
        } else {
            self.model_mappings.map_model(requested_model)
        };
        ModelMappingCollection::resolve_final_model(
            mapped.as_deref(),
            default_model,
            requested_model,
        )
    }

    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.with_timezone(&Utc)
    }

    pub fn updated_at_utc(&self) -> DateTime<Utc> {
        self.updated_at.with_timezone(&Utc)
    }
}

// ─── ModelEx 聚合根行为 ───────────────────────────────────────────

impl ModelEx {
    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.with_timezone(&Utc)
    }

    pub fn updated_at_utc(&self) -> DateTime<Utc> {
        self.updated_at.with_timezone(&Utc)
    }

    pub fn base_url(&self) -> Result<&str, AppError> {
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| AppError::Internal("Provider 未加载".to_string()))?;
        match self.api_type {
            AccessPointType::Anthropic => provider
                .anthropic_base_url
                .as_deref()
                .ok_or_else(|| AppError::Internal("提供商未配置 Anthropic 基础 URL".to_string())),
        }
    }

    pub fn resolve_model(&self, requested_model: &str) -> String {
        let default_model = self
            .provider
            .as_ref()
            .and_then(|p| p.default_model.as_deref());
        let mapped = if requested_model.is_empty() {
            None
        } else {
            self.model_mappings.map_model(requested_model)
        };
        ModelMappingCollection::resolve_final_model(
            mapped.as_deref(),
            default_model,
            requested_model,
        )
    }

    pub fn validate_usable(&self) -> Result<(), AppError> {
        if !self.status.is_enabled() {
            return Err(AppError::Forbidden(format!(
                "接入点 '{}' 已被禁用",
                self.short_code
            )));
        }
        if let Some(provider) = self.provider.as_ref() {
            if !provider.status.is_enabled() {
                return Err(AppError::Forbidden(format!(
                    "接入点 '{}' 关联的提供商已被禁用",
                    self.short_code
                )));
            }
        }
        if let Some(account) = self.account.as_ref() {
            if !account.status.is_enabled() {
                return Err(AppError::Forbidden(format!(
                    "接入点 '{}' 关联的账号已被禁用",
                    self.short_code
                )));
            }
        }
        Ok(())
    }

    pub async fn decrypt_upstream_key(
        &self,
        encryption_svc: &dyn EncryptionService,
    ) -> Result<String, AppError> {
        let account = self
            .account
            .as_ref()
            .ok_or_else(|| AppError::Internal("Account 未加载".to_string()))?;
        if account.api_key_encrypted.is_empty() {
            return Err(AppError::NotFound(
                "API Key 未正确存储，请删除并重新添加 Account".to_string(),
            ));
        }
        let decrypted = encryption_svc
            .decrypt(&account.api_key_encrypted)
            .await
            .map_err(|e| AppError::Encryption(e.to_string()))?;
        String::from_utf8(decrypted)
            .map_err(|_| AppError::Internal("API Key 解码失败: 非法的 UTF-8 格式".to_string()))
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::access_point::model_mapping::{
        ModelMapping, DEFAULT_MODEL_SENTINEL, UNMATCHED_MODEL_SENTINEL,
    };

    fn test_access_point() -> Model {
        Model::new(
            "test".to_string(),
            AccessPointType::Anthropic,
            ShortCode::generate(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        )
    }

    #[test]
    fn test_resolve_model_direct_match() {
        let mut ap = test_access_point();
        ap.model_mappings = vec![ModelMapping::new_exact(
            "gpt-4".to_string(),
            "gpt-4-turbo".to_string(),
        )]
        .into();
        assert_eq!(ap.resolve_model("gpt-4", None), "gpt-4-turbo");
    }

    #[test]
    fn test_resolve_model_unmatched_with_default() {
        let mut ap = test_access_point();
        ap.model_mappings = vec![ModelMapping::new_exact(
            UNMATCHED_MODEL_SENTINEL.to_string(),
            DEFAULT_MODEL_SENTINEL.to_string(),
        )]
        .into();
        assert_eq!(
            ap.resolve_model("unknown-model", Some("claude-sonnet")),
            "claude-sonnet"
        );
    }

    #[test]
    fn test_resolve_model_no_match_default_fallback() {
        let ap = test_access_point();
        assert_eq!(
            ap.resolve_model("unknown-model", Some("default-model")),
            "default-model"
        );
    }

    #[test]
    fn test_resolve_model_no_match_no_default() {
        let ap = test_access_point();
        assert_eq!(ap.resolve_model("my-model", None), "my-model");
    }

    #[test]
    fn test_resolve_model_empty_requested_with_default() {
        let ap = test_access_point();
        assert_eq!(ap.resolve_model("", Some("default")), "default");
    }

    #[test]
    fn test_resolve_model_empty_requested_no_default() {
        let ap = test_access_point();
        assert_eq!(ap.resolve_model("", None), "");
    }
}
