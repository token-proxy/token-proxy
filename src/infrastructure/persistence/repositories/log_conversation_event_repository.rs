use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::domain::entities::log_entry::LogConversationEvent;
use crate::domain::repositories::log_conversation_event_repository::LogConversationEventRepository;
use crate::infrastructure::persistence::entities::log_conversation_event::{
    ActiveModel, Column, Entity,
};
use crate::shared::error::AppError;

pub struct SeaOrmLogConversationEventRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmLogConversationEventRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl LogConversationEventRepository for SeaOrmLogConversationEventRepository {
    async fn save_many(&self, events: &[LogConversationEvent]) -> Result<(), AppError> {
        if events.is_empty() {
            return Ok(());
        }

        let models = events
            .iter()
            .cloned()
            .map(ActiveModel::from)
            .collect::<Vec<_>>();

        Entity::insert_many(models)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn find_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<Vec<LogConversationEvent>, AppError> {
        let models = Entity::find()
            .filter(Column::SessionId.eq(session_id))
            .order_by_asc(Column::RequestIndex)
            .order_by_asc(Column::EventIndex)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(LogConversationEvent::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn find_by_log_id(&self, log_id: Uuid) -> Result<Vec<LogConversationEvent>, AppError> {
        let models = Entity::find()
            .filter(Column::LogId.eq(log_id))
            .order_by_asc(Column::EventIndex)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(LogConversationEvent::try_from)
            .collect::<Result<Vec<_>, _>>()
    }
}
