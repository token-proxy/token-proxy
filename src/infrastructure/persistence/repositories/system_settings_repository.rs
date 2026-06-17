use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

use crate::domain::system::system_settings;
use crate::domain::system::SystemSettings;
use crate::domain::system::SystemSettingsRepository;
use crate::shared::error::AppError;

pub struct SeaOrmSystemSettingsRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmSystemSettingsRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        SeaOrmSystemSettingsRepository { db }
    }
}

#[async_trait]
impl SystemSettingsRepository for SeaOrmSystemSettingsRepository {
    async fn get(&self) -> Result<SystemSettings, AppError> {
        let row = system_settings::Entity::find_by_id(1i16)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match row {
            Some(ref model) => Ok(SystemSettings::from_model(model)),
            None => Ok(SystemSettings::default()),
        }
    }

    async fn save(&self, settings: &SystemSettings) -> Result<(), AppError> {
        // UPSERT: 使用 ActiveModel 设置 id=1，SeaORM 在存在时更新、不存在时插入
        let existing = system_settings::Entity::find_by_id(1i16)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let active = system_settings::ActiveModel {
            id: Set(1i16),
            log_retention_months: Set(settings.log_retention_months as i16),
            updated_at: Set(chrono::Utc::now().fixed_offset()),
        };

        if existing.is_some() {
            system_settings::Entity::update(active)
                .exec(&*self.db)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        } else {
            active
                .insert(&*self.db)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        Ok(())
    }
}
