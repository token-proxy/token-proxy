//! 系统设置应用服务 — application/system/
//!
//! 编排系统配置的读取和更新操作，包括日志保留月数等全局设置。

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::log::AuditAction;
use crate::domain::log::AuditEntityType;
use crate::domain::log::AuditLog;
use crate::domain::log::AuditLogRepository;
use crate::domain::system::SystemSettings;
use crate::domain::system::SystemSettingsRepository;
use crate::shared::error::AppError;

use super::dto::{SettingsResponse, UpdateSettingsRequest};

/// 系统设置应用服务
///
/// 编排系统全局配置的读取和更新，更新时记录审计日志。
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

    /// 获取系统设置
    pub async fn get_settings(&self) -> Result<SettingsResponse, AppError> {
        let settings = self.settings_repo.get().await?;
        Ok(SettingsResponse {
            log_retention_months: settings.log_retention_months as i16,
        })
    }

    /// 更新系统设置
    ///
    /// 校验日志保留月数范围（1-36），保存后记录审计日志。
    pub async fn update_settings(
        &self,
        input: UpdateSettingsRequest,
        user_id: Uuid,
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

        self.log_audit(
            Some(user_id),
            AuditAction::UpdateSettings,
            AuditEntityType::SystemSettings,
            None,
            Some(serde_json::json!({
                "log_retention_months": input.log_retention_months,
            })),
        )
        .await;

        Ok(SettingsResponse {
            log_retention_months: input.log_retention_months,
        })
    }

    /// fire-and-forget 审计日志写入
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
            tracing::error!(error = %e, action = %action, entity_type = %entity_type, "审计日志写入失败");
        }
    }
}
