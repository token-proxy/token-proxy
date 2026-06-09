use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

use crate::domain::provider::model_list::ModelList;
use crate::domain::shared::status::Status;
use crate::domain::shared::AccessPointType;
use crate::shared::error::AppError;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "providers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub openai_base_url: Option<String>,
    pub anthropic_base_url: Option<String>,
    pub models: ModelList,
    pub status: Status,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,

    #[sea_orm(has_many)]
    pub accounts: HasMany<super::account::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
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

        let offset = FixedOffset::east_opt(0).expect("UTC offset");
        let now = Utc::now().with_timezone(&offset);
        Ok(Model {
            id: Uuid::new_v4(),
            name,
            openai_base_url: openai_base_url
                .map(|u| u.trim().to_string())
                .filter(|u| !u.is_empty()),
            anthropic_base_url: anthropic_base_url
                .map(|u| u.trim().to_string())
                .filter(|u| !u.is_empty()),
            models: ModelList::default(),
            status: Status::Enabled,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn created_at_utc(&self) -> DateTime<Utc> {
        self.created_at.with_timezone(&Utc)
    }

    pub fn updated_at_utc(&self) -> DateTime<Utc> {
        self.updated_at.with_timezone(&Utc)
    }

    pub fn base_url_for(&self, api_type: &AccessPointType) -> Result<&str, AppError> {
        match api_type {
            AccessPointType::Anthropic => self
                .anthropic_base_url
                .as_deref()
                .ok_or_else(|| AppError::Internal("提供商未配置 Anthropic 基础 URL".to_string())),
        }
    }

    /// 重命名，名称不可为空
    pub fn rename(&mut self, name: String) -> Result<(), AppError> {
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            return Err(AppError::Validation("提供商名称不能为空".to_string()));
        }
        self.name = trimmed;
        self.touch();
        Ok(())
    }

    /// 替换模型列表，自动排序去重
    pub fn set_models(&mut self, models: Vec<String>) {
        let mut list: ModelList = models.into();
        list.sort();
        list.dedup();
        self.models = list;
        self.touch();
    }

    /// 合并模型列表（自动排序去重），供自动发现使用
    pub fn merge_models(&mut self, new_models: Vec<String>) {
        self.models.extend(new_models);
        self.models.sort();
        self.models.dedup();
        self.touch();
    }

    /// 启用
    pub fn enable(&mut self) {
        self.status = Status::Enabled;
        self.touch();
    }

    /// 禁用
    pub fn disable(&mut self) {
        self.status = Status::Disabled;
        self.touch();
    }

    /// 更新 updated_at 为当前时间
    fn touch(&mut self) {
        self.updated_at = chrono::Utc::now()
            .with_timezone(&chrono::FixedOffset::east_opt(0).expect("UTC offset"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_new_valid() {
        let provider = Model::new(
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
        let result = Model::new(
            "  ".to_string(),
            None,
            Some("https://api.anthropic.com".to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_new_no_url() {
        let result = Model::new("Test".to_string(), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_new_only_openai() {
        let provider = Model::new(
            "OpenAI Only".to_string(),
            Some("https://api.openai.com".to_string()),
            None,
        )
        .unwrap();
        assert!(provider.anthropic_base_url.is_none());
    }
}
