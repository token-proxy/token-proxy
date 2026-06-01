use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::value_objects::status::Status;
use crate::shared::error::AppError;

/// SeaORM 实体映射 providers 表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "providers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
    #[sea_orm(column_type = "JsonBinary")]
    pub models: Vec<String>,
    pub default_model: Option<String>,
    pub status: Status,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::account::Entity")]
    Account,
}

impl ActiveModelBehavior for ActiveModel {}

/// 领域实体 Provider
pub type Provider = Model;

// ─── 领域行为 ──────────────────────────────────────────────────────

impl Model {
    /// 创建新的 Provider，执行领域校验
    pub fn new(
        name: String,
        openai_base_url: Option<String>,
        anthropic_base_url: Option<String>,
        default_model: Option<String>,
    ) -> Result<Self, AppError> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Validation("提供商名称不能为空".to_string()));
        }

        let has_openai = openai_base_url
            .as_ref()
            .map(|u| !u.trim().is_empty())
            .unwrap_or(false);
        let has_anthropic = anthropic_base_url
            .as_ref()
            .map(|u| !u.trim().is_empty())
            .unwrap_or(false);

        if !has_openai && !has_anthropic {
            return Err(AppError::Validation(
                "至少需要提供 OpenAI 和 Anthropic 中的一个基础 URL".to_string(),
            ));
        }

        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        let now = Utc::now().with_timezone(&offset);
        Ok(Provider {
            id: Uuid::new_v4(),
            name,
            openai_base_url: openai_base_url
                .map(|u| u.trim().to_string())
                .filter(|u| !u.is_empty()),
            anthropic_base_url: anthropic_base_url
                .map(|u| u.trim().to_string())
                .filter(|u| !u.is_empty()),
            models: Vec::new(),
            default_model: default_model
                .map(|m| m.trim().to_string())
                .filter(|m| !m.is_empty()),
            status: Status::Enabled,
            created_at: now,
            updated_at: now,
        })
    }

    /// 获取 created_at 为 DateTime<Utc>
    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.with_timezone(&Utc)
    }

    /// 获取 updated_at 为 DateTime<Utc>
    pub fn updated_at_utc(&self) -> DateTime<Utc> {
        self.updated_at.with_timezone(&Utc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_new_valid() {
        let provider = Provider::new(
            "Test Provider".to_string(),
            Some("https://api.openai.com".to_string()),
            Some("https://api.anthropic.com".to_string()),
            Some("gpt-4".to_string()),
        )
        .unwrap();
        assert_eq!(provider.name, "Test Provider");
        assert_eq!(
            provider.openai_base_url,
            Some("https://api.openai.com".to_string())
        );
        assert_eq!(
            provider.anthropic_base_url,
            Some("https://api.anthropic.com".to_string())
        );
        assert_eq!(provider.default_model, Some("gpt-4".to_string()));
        assert!(provider.status.is_enabled());
    }

    #[test]
    fn test_provider_new_default_model_empty() {
        let provider = Provider::new(
            "Test".to_string(),
            Some("https://api.openai.com".to_string()),
            None,
            Some("  ".to_string()),
        )
        .unwrap();
        assert_eq!(provider.default_model, None);
    }

    #[test]
    fn test_provider_new_default_model_none() {
        let provider = Provider::new(
            "Test".to_string(),
            Some("https://api.openai.com".to_string()),
            None,
            None,
        )
        .unwrap();
        assert_eq!(provider.default_model, None);
    }

    #[test]
    fn test_provider_new_empty_name() {
        let result = Provider::new(
            "  ".to_string(),
            None,
            Some("https://api.anthropic.com".to_string()),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_new_no_url() {
        let result = Provider::new("Test".to_string(), None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_new_only_openai() {
        let provider = Provider::new(
            "OpenAI Only".to_string(),
            Some("https://api.openai.com".to_string()),
            None,
            None,
        )
        .unwrap();
        assert!(provider.anthropic_base_url.is_none());
    }
}
