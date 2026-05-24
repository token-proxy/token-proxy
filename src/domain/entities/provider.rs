use crate::domain::value_objects::status::Status;
use crate::shared::error::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: Uuid,
    pub name: String,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
    pub models: Vec<String>,
    pub status: Status,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Provider {
    /// 创建新的 Provider，执行领域校验
    pub fn new(
        name: String,
        openai_base_url: Option<String>,
        anthropic_base_url: Option<String>,
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

        let now = Utc::now();
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
            status: Status::Enabled,
            created_at: now,
            updated_at: now,
        })
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
        assert!(provider.status.is_enabled());
    }

    #[test]
    fn test_provider_new_empty_name() {
        let result = Provider::new(
            "  ".to_string(),
            None,
            Some("https://api.anthropic.com".to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_new_no_url() {
        let result = Provider::new("Test".to_string(), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_new_only_openai() {
        let provider = Provider::new(
            "OpenAI Only".to_string(),
            Some("https://api.openai.com".to_string()),
            None,
        )
        .unwrap();
        assert!(provider.anthropic_base_url.is_none());
    }
}
