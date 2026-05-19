use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};

use crate::domain::entities::account::Account;
use crate::domain::repositories::account_repository::AccountRepository;
use crate::infrastructure::persistence::entities::account::{ActiveModel, Column, Entity};
use crate::shared::error::AppError;
use uuid::Uuid;

pub struct SeaOrmAccountRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmAccountRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        SeaOrmAccountRepository { db }
    }
}

#[async_trait]
impl AccountRepository for SeaOrmAccountRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Account>, AppError> {
        let db = &*self.db;
        let model = Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match model {
            Some(m) => Ok(Some(m.try_into()?)),
            None => Ok(None),
        }
    }

    async fn find_by_provider_id(&self, provider_id: Uuid) -> Result<Vec<Account>, AppError> {
        let db = &*self.db;
        let models = Entity::find()
            .filter(Column::ProviderId.eq(provider_id))
            .all(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<Account>, AppError>>()
    }

    async fn find_enabled_by_provider_id(&self, provider_id: Uuid) -> Result<Vec<Account>, AppError> {
        let db = &*self.db;
        let models = Entity::find()
            .filter(Column::ProviderId.eq(provider_id))
            .filter(Column::Status.eq("enabled"))
            .all(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<Account>, AppError>>()
    }

    async fn get_encrypted_api_key(&self, account_id: Uuid) -> Result<Vec<u8>, AppError> {
        let db = &*self.db;
        let model = Entity::find_by_id(account_id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("账号 {} 未找到", account_id)))?;

        Ok(model.api_key_encrypted)
    }

    async fn find_all(&self) -> Result<Vec<Account>, AppError> {
        let db = &*self.db;
        let models = Entity::find()
            .all(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<Account>, AppError>>()
    }

    async fn save(&self, account: &Account) -> Result<Account, AppError> {
        let db = &*self.db;
        let exists = Entity::find_by_id(account.id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .is_some();

        // Account 的 ActiveModel 需要 api_key_encrypted, 但领域实体不包含此字段。
        // 更新时保持原加密数据不变; 新建时使用空 Vec（应由应用层补充处理）。
        use chrono::FixedOffset;
        let offset = FixedOffset::east_opt(0).expect("UTC offset");

        let active_model = if exists {
            let existing = Entity::find_by_id(account.id)
                .one(db)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?
                .ok_or_else(|| AppError::NotFound("Account 不存在".to_string()))?;

            ActiveModel {
                id: Set(account.id),
                provider_id: Set(account.provider_id),
                name: Set(account.name.clone()),
                api_key_encrypted: Set(existing.api_key_encrypted),
                api_key_suffix: Set(account.api_key_suffix.clone()),
                status: Set(account.status.to_string()),
                created_at: Set(account.created_at.with_timezone(&offset)),
                updated_at: Set(account.updated_at.with_timezone(&offset)),
            }
        } else {
            ActiveModel {
                id: Set(account.id),
                provider_id: Set(account.provider_id),
                name: Set(account.name.clone()),
                api_key_encrypted: Set(Vec::new()),
                api_key_suffix: Set(account.api_key_suffix.clone()),
                status: Set(account.status.to_string()),
                created_at: Set(account.created_at.with_timezone(&offset)),
                updated_at: Set(account.updated_at.with_timezone(&offset)),
            }
        };

        if exists {
            Entity::update(active_model)
                .exec(db)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        } else {
            Entity::insert(active_model)
                .exec(db)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        Entity::find_by_id(account.id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .map(|m| m.try_into())
            .ok_or_else(|| AppError::Internal("保存后无法查询到 Account".to_string()))?
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let db = &*self.db;
        Entity::delete_by_id(id)
            .exec(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}