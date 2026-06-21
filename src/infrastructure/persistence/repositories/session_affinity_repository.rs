//! 会话粘滞 Repository 实现（基础设施层）
//!
//! 管理 `session_affinity` 表，核心运作是：
//! - 查询指定接入点和 session 的绑定
//! - upsert 绑定关系（使用 PostgreSQL 的 INSERT ... ON CONFLICT DO UPDATE）
//! - 清理过期绑定

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Statement,
};
use uuid::Uuid;

use crate::domain::access_point::{SessionAffinity, SessionAffinityRepository};
use crate::shared::error::AppError;

// ─── SeaORM 实体 (session_affinity 表) ─────────────────────────────

/// 会话粘滞绑定记录
///
/// 绑定一个 session 到指定接入点下的一个账号，
/// 确保同一会话的请求始终路由到同一账号。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "session_affinity")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub access_point_id: Uuid,
    pub session_id: String,
    pub account_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::domain::access_point::access_point::Entity",
        from = "Column::AccessPointId",
        to = "crate::domain::access_point::access_point::Column::Id"
    )]
    AccessPoint,
    #[sea_orm(
        belongs_to = "crate::domain::provider::account::Entity",
        from = "Column::AccountId",
        to = "crate::domain::provider::account::Column::Id"
    )]
    Account,
}

impl ActiveModelBehavior for ActiveModel {}

// ─── Model ↔ SessionAffinity 映射 ───────────────────────────────────

impl From<Model> for SessionAffinity {
    fn from(m: Model) -> Self {
        SessionAffinity {
            id: m.id,
            access_point_id: m.access_point_id,
            session_id: m.session_id,
            account_id: m.account_id,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

// ─── Repository 实现 ────────────────────────────────────────────────

pub struct SeaOrmSessionAffinityRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmSessionAffinityRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        SeaOrmSessionAffinityRepository { db }
    }
}

#[async_trait]
impl SessionAffinityRepository for SeaOrmSessionAffinityRepository {
    async fn find_by_access_point_and_session(
        &self,
        access_point_id: Uuid,
        session_id: &str,
    ) -> Result<Option<SessionAffinity>, AppError> {
        let model = Entity::find()
            .filter(Column::AccessPointId.eq(access_point_id))
            .filter(Column::SessionId.eq(session_id))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::Database(format!("查询会话绑定失败: {}", e)))?;
        Ok(model.map(Into::into))
    }

    async fn upsert(
        &self,
        access_point_id: Uuid,
        session_id: &str,
        account_id: Uuid,
    ) -> Result<SessionAffinity, AppError> {
        // 使用 INSERT ... ON CONFLICT DO UPDATE 实现 upsert
        let sql = r#"
            INSERT INTO session_affinity (id, access_point_id, session_id, account_id, created_at, updated_at)
            VALUES (gen_random_uuid(), $1, $2, $3, NOW(), NOW())
            ON CONFLICT (access_point_id, session_id)
            DO UPDATE SET account_id = $3, updated_at = NOW()
            RETURNING id, access_point_id, session_id, account_id, created_at, updated_at
        "#;

        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            vec![access_point_id.into(), session_id.into(), account_id.into()],
        );

        let result = self
            .db
            .query_one_raw(stmt)
            .await
            .map_err(|e| AppError::Database(format!("会话绑定 upsert 失败: {}", e)))?;

        match result {
            Some(row) => {
                let id: Uuid = row
                    .try_get_by_index(0)
                    .map_err(|e| AppError::Internal(format!("解析 id 失败: {}", e)))?;
                let ap_id: Uuid = row
                    .try_get_by_index(1)
                    .map_err(|e| AppError::Internal(format!("解析 access_point_id 失败: {}", e)))?;
                let sid: String = row
                    .try_get_by_index(2)
                    .map_err(|e| AppError::Internal(format!("解析 session_id 失败: {}", e)))?;
                let acc_id: Uuid = row
                    .try_get_by_index(3)
                    .map_err(|e| AppError::Internal(format!("解析 account_id 失败: {}", e)))?;
                let created_at: DateTimeWithTimeZone = row
                    .try_get_by_index(4)
                    .map_err(|e| AppError::Internal(format!("解析 created_at 失败: {}", e)))?;
                let updated_at: DateTimeWithTimeZone = row
                    .try_get_by_index(5)
                    .map_err(|e| AppError::Internal(format!("解析 updated_at 失败: {}", e)))?;

                Ok(SessionAffinity {
                    id,
                    access_point_id: ap_id,
                    session_id: sid,
                    account_id: acc_id,
                    created_at,
                    updated_at,
                })
            }
            None => Err(AppError::Internal("会话绑定 upsert 未返回记录".to_string())),
        }
    }

    async fn delete_stale(&self, older_than: chrono::Duration) -> Result<u64, AppError> {
        let cutoff = chrono::Utc::now() - older_than;
        let result = Entity::delete_many()
            .filter(Column::UpdatedAt.lt(cutoff))
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::Database(format!("清理过期会话绑定失败: {}", e)))?;
        Ok(result.rows_affected)
    }
}
