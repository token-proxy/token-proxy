use async_trait::async_trait;

use super::system_settings::SystemSettings;
use crate::shared::error::AppError;

/// 系统设置仓储接口
#[async_trait]
pub trait SystemSettingsRepository: Send + Sync {
    /// 获取当前设置，不存在时返回默认值
    async fn get(&self) -> Result<SystemSettings, AppError>;

    /// 保存设置（upsert）
    async fn save(&self, settings: &SystemSettings) -> Result<(), AppError>;
}
