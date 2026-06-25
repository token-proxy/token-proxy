//! 接入点 Repository 实现（基础设施层）

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};

use crate::domain::access_point::access_point_account::{
    AccessPointAccount, AccessPointAccountDetail,
};
use crate::domain::access_point::repository::AccessPointRepository;
use crate::domain::access_point::AccessPoint;
use crate::domain::access_point::{
    AccessPointActiveModel, AccessPointColumn, AccessPointEntity, AccessPointEx,
};
use crate::domain::shared::Status;
use crate::shared::error::AppError;
use uuid::Uuid;

use super::access_point_account_repository as accounts_mod;

pub struct SeaOrmAccessPointRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmAccessPointRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        SeaOrmAccessPointRepository { db }
    }
}

#[async_trait]
impl AccessPointRepository for SeaOrmAccessPointRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AccessPoint>, AppError> {
        Ok(AccessPointEntity::find_by_id(id).one(&*self.db).await?)
    }

    async fn find_by_short_code(
        &self,
        short_code: &str,
    ) -> Result<Option<AccessPointEx>, AppError> {
        let ap = AccessPointEntity::find()
            .filter(AccessPointColumn::ShortCode.eq(short_code))
            .one(&*self.db)
            .await?;

        match ap {
            Some(ap_model) => {
                // 加载账户池
                let accounts: Vec<AccessPointAccount> = accounts_mod::Entity::find()
                    .filter(accounts_mod::Column::AccessPointId.eq(ap_model.id))
                    .order_by_asc(accounts_mod::Column::Priority)
                    .all(&*self.db)
                    .await?
                    .into_iter()
                    .map(|a| AccessPointAccount {
                        account_id: a.account_id,
                        weight: a.weight,
                        priority: a.priority,
                    })
                    .collect();

                Ok(Some(AccessPointEx::from_model(ap_model, accounts)))
            }
            None => Ok(None),
        }
    }

    async fn find_enabled(&self) -> Result<Vec<AccessPoint>, AppError> {
        Ok(AccessPointEntity::find()
            .filter(AccessPointColumn::Status.eq(Status::Enabled))
            .all(&*self.db)
            .await?)
    }

    async fn find_by_provider_id(&self, provider_id: Uuid) -> Result<Vec<AccessPoint>, AppError> {
        // AccessPoint 不再直接关联 Provider，通过 accounts → access_point_accounts 间接查找
        let account_ids: Vec<Uuid> = crate::domain::provider::AccountEntity::find()
            .filter(crate::domain::provider::AccountColumn::ProviderId.eq(provider_id))
            .all(&*self.db)
            .await?
            .into_iter()
            .map(|a| a.id)
            .collect();

        if account_ids.is_empty() {
            return Ok(Vec::new());
        }

        let ap_ids: Vec<Uuid> = accounts_mod::Entity::find()
            .filter(accounts_mod::Column::AccountId.is_in(account_ids))
            .all(&*self.db)
            .await?
            .into_iter()
            .map(|a| a.access_point_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if ap_ids.is_empty() {
            return Ok(Vec::new());
        }

        Ok(AccessPointEntity::find()
            .filter(AccessPointColumn::Id.is_in(ap_ids))
            .all(&*self.db)
            .await?)
    }

    async fn find_by_account_id(&self, account_id: Uuid) -> Result<Vec<AccessPoint>, AppError> {
        let ap_ids: Vec<Uuid> = accounts_mod::Entity::find()
            .filter(accounts_mod::Column::AccountId.eq(account_id))
            .all(&*self.db)
            .await?
            .into_iter()
            .map(|a| a.access_point_id)
            .collect();

        if ap_ids.is_empty() {
            return Ok(Vec::new());
        }

        Ok(AccessPointEntity::find()
            .filter(AccessPointColumn::Id.is_in(ap_ids))
            .all(&*self.db)
            .await?)
    }

    async fn find_all(&self) -> Result<Vec<AccessPoint>, AppError> {
        Ok(AccessPointEntity::find().all(&*self.db).await?)
    }

    async fn find_by_created_by(&self, created_by: Uuid) -> Result<Vec<AccessPoint>, AppError> {
        Ok(AccessPointEntity::find()
            .filter(AccessPointColumn::CreatedBy.eq(created_by))
            .order_by_desc(AccessPointColumn::CreatedAt)
            .all(&*self.db)
            .await?)
    }

    async fn save(&self, access_point: &AccessPoint) -> Result<AccessPoint, AppError> {
        let db = &*self.db;
        let exists = AccessPointEntity::find_by_id(access_point.id)
            .one(db)
            .await?
            .is_some();

        let active_model: AccessPointActiveModel = access_point.clone().into();

        if exists {
            let active_model = active_model.reset_all();
            AccessPointEntity::update(active_model).exec(db).await?;
        } else {
            AccessPointEntity::insert(active_model).exec(db).await?;
        }

        AccessPointEntity::find_by_id(access_point.id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Internal("保存后无法查询到 AccessPoint".to_string()))
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        AccessPointEntity::delete_by_id(id).exec(&*self.db).await?;
        Ok(())
    }

    async fn find_accounts_by_access_point(
        &self,
        access_point_id: Uuid,
    ) -> Result<Vec<AccessPointAccount>, AppError> {
        let accounts = accounts_mod::Entity::find()
            .filter(accounts_mod::Column::AccessPointId.eq(access_point_id))
            .order_by_asc(accounts_mod::Column::Priority)
            .all(&*self.db)
            .await?;

        Ok(accounts
            .into_iter()
            .map(|a| AccessPointAccount {
                account_id: a.account_id,
                weight: a.weight,
                priority: a.priority,
            })
            .collect())
    }

    async fn find_account_details_by_access_point(
        &self,
        access_point_id: Uuid,
    ) -> Result<Vec<AccessPointAccountDetail>, AppError> {
        // 1. 查询接入点关联的账户池条目
        let links = accounts_mod::Entity::find()
            .filter(accounts_mod::Column::AccessPointId.eq(access_point_id))
            .order_by_asc(accounts_mod::Column::Priority)
            .all(&*self.db)
            .await?;

        // 2. 收集 account_id 并批量查询 accounts 表
        let account_ids: Vec<Uuid> = links.iter().map(|l| l.account_id).collect();
        let accounts = if account_ids.is_empty() {
            Vec::new()
        } else {
            crate::domain::provider::AccountEntity::find()
                .filter(crate::domain::provider::AccountColumn::Id.is_in(account_ids))
                .all(&*self.db)
                .await?
        };

        // 3. 按 account_id 查找账号信息并合并
        let detail = links
            .into_iter()
            .map(|link| {
                let acct = accounts.iter().find(|a| a.id == link.account_id);
                AccessPointAccountDetail {
                    account_id: link.account_id,
                    provider_id: acct.map_or(Uuid::nil(), |a| a.provider_id),
                    weight: link.weight,
                    priority: link.priority,
                    status: acct.map_or_else(|| "unknown".to_string(), |a| a.status.to_string()),
                }
            })
            .collect();

        Ok(detail)
    }

    async fn save_accounts(
        &self,
        access_point_id: Uuid,
        accounts: &[AccessPointAccount],
    ) -> Result<(), AppError> {
        let db = Arc::clone(&self.db);
        let ap_id = access_point_id;
        let accounts_vec = accounts.to_vec();

        db.transaction(|txn| {
            let accounts_vec = accounts_vec.clone();
            Box::pin(async move {
                // 先删除该接入点的所有账户
                accounts_mod::Entity::delete_many()
                    .filter(accounts_mod::Column::AccessPointId.eq(ap_id))
                    .exec(txn)
                    .await?;

                // 再插入新记录
                for entry in accounts_vec {
                    let model = accounts_mod::ActiveModel {
                        id: Set(Uuid::new_v4()),
                        access_point_id: Set(ap_id),
                        account_id: Set(entry.account_id),
                        weight: Set(entry.weight),
                        priority: Set(entry.priority),
                        created_at: Set(chrono::Utc::now().into()),
                    };
                    accounts_mod::Entity::insert(model).exec(txn).await?;
                }

                Ok(())
            })
        })
        .await
        .map_err(|e: sea_orm::TransactionError<sea_orm::DbErr>| match e {
            sea_orm::TransactionError::Connection(db_err)
            | sea_orm::TransactionError::Transaction(db_err) => AppError::from(db_err),
        })?;

        Ok(())
    }
}
