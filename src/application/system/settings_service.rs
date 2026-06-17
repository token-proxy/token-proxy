use std::sync::Arc;

use crate::domain::log::AuditLog;
use crate::domain::log::AuditLogRepository;
use crate::domain::system::SystemSettings;
use crate::domain::system::SystemSettingsRepository;
use crate::shared::error::AppError;

use super::dto::{SettingsResponse, UpdateSettingsRequest};

pub struct SettingsService {
    settings_repo: Arc<dyn SystemSettingsRepository>,
    audit_log_repo: Arc<dyn AuditLogRepository>,
}

impl SettingsService {
    pub fn new(
        settings_repo: Arc<dyn SystemSettingsRepository>,
        audit_log_repo: Arc<dyn AuditLogRepository>,
    ) -> Self {
        SettingsService {
            settings_repo,
            audit_log_repo,
        }
    }

    pub async fn get_settings(&self) -> Result<SettingsResponse, AppError> {
        let settings = self.settings_repo.get().await?;
        Ok(SettingsResponse {
            log_retention_months: settings.log_retention_months as i16,
        })
    }

    pub async fn update_settings(
        &self,
        input: UpdateSettingsRequest,
        user_id: Option<uuid::Uuid>,
    ) -> Result<SettingsResponse, AppError> {
        // 校验范围
        if !(1..=36).contains(&input.log_retention_months) {
            return Err(AppError::Validation(
                "日志保留月数必须在 1 到 36 之间".to_string(),
            ));
        }

        let settings = SystemSettings {
            log_retention_months: input.log_retention_months as u32,
        };

        self.settings_repo.save(&settings).await?;

        // 审计日志（忽略写入失败）
        let audit = AuditLog::new(
            user_id,
            "update",
            "system_settings",
            Some(uuid::Uuid::new_v4()),
            Some(serde_json::json!({
                "log_retention_months": input.log_retention_months,
            })),
        );
        if let Err(e) = self.audit_log_repo.save(&audit).await {
            tracing::error!(error = %e, "系统设置审计日志写入失败");
        }

        Ok(SettingsResponse {
            log_retention_months: input.log_retention_months,
        })
    }
}
