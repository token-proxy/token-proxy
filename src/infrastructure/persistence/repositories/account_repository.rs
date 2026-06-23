//! 账号 Repository 实现（基础设施层）

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::domain::provider::repository::AccountRepository;
use crate::domain::provider::Account;
use crate::domain::provider::{AccountActiveModel, AccountColumn, AccountEntity};
use crate::domain::shared::Status;
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
        Ok(AccountEntity::find_by_id(id).one(&*self.db).await?)
    }

    async fn find_by_provider_id(&self, provider_id: Uuid) -> Result<Vec<Account>, AppError> {
        Ok(AccountEntity::find()
            .filter(AccountColumn::ProviderId.eq(provider_id))
            .all(&*self.db)
            .await?)
    }

    async fn find_enabled_by_provider_id(
        &self,
        provider_id: Uuid,
    ) -> Result<Vec<Account>, AppError> {
        Ok(AccountEntity::find()
            .filter(AccountColumn::ProviderId.eq(provider_id))
            .filter(AccountColumn::Status.eq(Status::Enabled))
            .all(&*self.db)
            .await?)
    }

    async fn get_encrypted_api_key(&self, account_id: Uuid) -> Result<Vec<u8>, AppError> {
        let db = &*self.db;
        let model = AccountEntity::find_by_id(account_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("账号 {} 未找到", account_id)))?;

        Ok(model.api_key_encrypted)
    }

    async fn find_all(&self) -> Result<Vec<Account>, AppError> {
        Ok(AccountEntity::find().all(&*self.db).await?)
    }

    async fn save(&self, account: &Account) -> Result<Account, AppError> {
        let db = &*self.db;
        let exists = AccountEntity::find_by_id(account.id)
            .one(db)
            .await?
            .is_some();

        // Account 的 AccountActiveModel 需要 api_key_encrypted, 但领域实体不包含此字段。
        // 更新时保持原加密数据不变; 新建时使用空 Vec（应由应用层补充处理）。

        let active_model = if exists {
            let existing = AccountEntity::find_by_id(account.id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Account 不存在".to_string()))?;

            AccountActiveModel {
                id: Set(account.id),
                provider_id: Set(account.provider_id),
                name: Set(account.name.clone()),
                api_key_encrypted: Set(existing.api_key_encrypted),
                api_key_suffix: Set(account.api_key_suffix.clone()),
                disabled_reason: Set(account.disabled_reason.clone()),
                available_at: Set(account.available_at),
                status: Set(account.status.clone()),
                created_at: Set(account.created_at),
                updated_at: Set(account.updated_at),
            }
        } else {
            AccountActiveModel {
                id: Set(account.id),
                provider_id: Set(account.provider_id),
                name: Set(account.name.clone()),
                api_key_encrypted: Set(Vec::new()),
                api_key_suffix: Set(account.api_key_suffix.clone()),
                disabled_reason: Set(account.disabled_reason.clone()),
                available_at: Set(account.available_at),
                status: Set(account.status.clone()),
                created_at: Set(account.created_at),
                updated_at: Set(account.updated_at),
            }
        };

        if exists {
            AccountEntity::update(active_model).exec(db).await?;
        } else {
            AccountEntity::insert(active_model).exec(db).await?;
        }

        AccountEntity::find_by_id(account.id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Internal("保存后无法查询到 Account".to_string()))
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let db = &*self.db;
        AccountEntity::delete_by_id(id).exec(db).await?;
        Ok(())
    }

    /// 批量恢复已到恢复时间的自动禁用账号。
    ///
    /// 单条 UPDATE ... RETURNING id 完成：
    /// 将满足条件的账号重置为启用状态，返回被恢复的账号 ID 列表。
    async fn recover_expired_auto_disabled(&self) -> Result<Vec<Uuid>, AppError> {
        use sea_orm::{DbBackend, FromQueryResult, Statement};

        let db = &*self.db;
        let sql = r#"
            UPDATE accounts
            SET status           = 'enabled',
                disabled_reason  = NULL,
                available_at     = NULL,
                updated_at       = NOW()
            WHERE status          = 'disabled'
              AND disabled_reason IN ('rate_limited', 'balance_exhausted', 'fault')
              AND available_at   IS NOT NULL
              AND available_at   <= NOW()
            RETURNING id
        "#;

        #[derive(Debug, FromQueryResult)]
        struct IdRow {
            id: Uuid,
        }

        let rows =
            IdRow::find_by_statement(Statement::from_sql_and_values(DbBackend::Postgres, sql, []))
                .all(db)
                .await?;

        Ok(rows.into_iter().map(|r| r.id).collect())
    }

    async fn save_with_encrypted_key(
        &self,
        account: &Account,
        encrypted_api_key: &[u8],
    ) -> Result<Account, AppError> {
        let db = &*self.db;

        let active_model = AccountActiveModel {
            id: Set(account.id),
            provider_id: Set(account.provider_id),
            name: Set(account.name.clone()),
            api_key_encrypted: Set(encrypted_api_key.to_vec()),
            api_key_suffix: Set(account.api_key_suffix.clone()),
            disabled_reason: Set(account.disabled_reason.clone()),
            available_at: Set(account.available_at),
            status: Set(account.status.clone()),
            created_at: Set(account.created_at),
            updated_at: Set(account.updated_at),
        };

        AccountEntity::insert(active_model).exec(db).await?;

        AccountEntity::find_by_id(account.id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Internal("保存后无法查询到 Account".to_string()))
    }

    async fn update_encrypted_api_key(
        &self,
        account_id: Uuid,
        encrypted_api_key: &[u8],
    ) -> Result<(), AppError> {
        let db = &*self.db;
        let existing = AccountEntity::find_by_id(account_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("账号 {} 未找到", account_id)))?;

        use chrono::Utc;

        let active_model = AccountActiveModel {
            id: Set(existing.id),
            api_key_encrypted: Set(encrypted_api_key.to_vec()),
            updated_at: Set(Utc::now().fixed_offset()),
            // 其他字段保持不变
            provider_id: ActiveValue::NotSet,
            name: ActiveValue::NotSet,
            api_key_suffix: ActiveValue::NotSet,
            disabled_reason: ActiveValue::NotSet,
            available_at: ActiveValue::NotSet,
            status: ActiveValue::NotSet,
            created_at: ActiveValue::NotSet,
        };

        AccountEntity::update(active_model).exec(db).await?;

        Ok(())
    }
}
