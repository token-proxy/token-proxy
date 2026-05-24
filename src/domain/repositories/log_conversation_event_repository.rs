use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::log_entry::LogConversationEvent;
use crate::shared::error::AppError;

#[async_trait]
pub trait LogConversationEventRepository: Send + Sync {
    async fn save_many(&self, events: &[LogConversationEvent]) -> Result<(), AppError>;

    async fn find_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<Vec<LogConversationEvent>, AppError>;

    async fn find_by_log_id(&self, log_id: Uuid) -> Result<Vec<LogConversationEvent>, AppError>;
}
