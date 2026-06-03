use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::log::LogTokenUsage;
use crate::shared::error::AppError;

#[async_trait]
pub trait LogTokenUsageRepository: Send + Sync {
    async fn save(&self, usage: &LogTokenUsage) -> Result<(), AppError>;

    async fn find_by_log_id(&self, log_id: Uuid) -> Result<Option<LogTokenUsage>, AppError>;

    async fn find_by_session_id(&self, session_id: &str) -> Result<Vec<LogTokenUsage>, AppError>;
}
