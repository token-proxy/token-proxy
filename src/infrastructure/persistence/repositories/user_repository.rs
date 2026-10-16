//! 用户 Repository 实现（基础设施层）

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, Statement,
};

use crate::domain::shared::Status;
use crate::domain::user::User;
use crate::domain::user::UserDependents;
use crate::domain::user::UserRepository;
use crate::domain::user::{UserActiveModel, UserColumn, UserEntity};
use crate::shared::error::AppError;
use uuid::Uuid;

pub struct SeaOrmUserRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmUserRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        SeaOrmUserRepository { db }
    }
}

#[async_trait]
impl UserRepository for SeaOrmUserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AppError> {
        Ok(UserEntity::find_by_id(id).one(&*self.db).await?)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        Ok(UserEntity::find()
            .filter(UserColumn::Username.eq(username))
            .one(&*self.db)
            .await?)
    }

    async fn exists_by_username(&self, username: &str) -> Result<bool, AppError> {
        Ok(UserEntity::find()
            .filter(UserColumn::Username.eq(username))
            .count(&*self.db)
            .await?
            > 0)
    }

    async fn find_all(&self) -> Result<Vec<User>, AppError> {
        Ok(UserEntity::find().all(&*self.db).await?)
    }

    async fn save(&self, user: &User) -> Result<User, AppError> {
        let db = &*self.db;
        let exists = UserEntity::find_by_id(user.id).one(db).await?.is_some();

        let active_model: UserActiveModel = user.clone().into();

        if exists {
            let active_model = active_model.reset_all();
            UserEntity::update(active_model).exec(db).await?;
        } else {
            UserEntity::insert(active_model).exec(db).await?;
        }

        UserEntity::find_by_id(user.id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Internal("保存后无法查询到 User".to_string()))
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        UserEntity::delete_by_id(id).exec(&*self.db).await?;
        Ok(())
    }

    /// 统计关联数据量：用户创建的接入点、其请求日志、其操作审计
    ///
    /// 三个子查询一次往返（与本项目 `log_repository` 的原生 SQL 风格一致）；
    /// 接入点是 ON DELETE RESTRICT 的外键，日志与审计无外键但属审计线索。
    async fn count_dependents(&self, id: Uuid) -> Result<UserDependents, AppError> {
        const SQL: &str = r#"
            SELECT
                (SELECT COUNT(*) FROM access_points WHERE created_by = $1::uuid),
                (SELECT COUNT(*) FROM log_requests  WHERE user_id     = $1::uuid),
                (SELECT COUNT(*) FROM audit_logs    WHERE operator_id = $1::uuid)
        "#;
        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, SQL, [id.into()]);
        let row = self
            .db
            .query_one_raw(stmt)
            .await?
            .ok_or_else(|| AppError::Internal("统计用户关联数据失败".to_string()))?;
        Ok(UserDependents {
            access_points: row.try_get_by_index::<i64>(0)?,
            logs: row.try_get_by_index::<i64>(1)?,
            audits: row.try_get_by_index::<i64>(2)?,
        })
    }

    /// 统计启用状态用户数量
    async fn count_enabled(&self) -> Result<i64, AppError> {
        Ok(UserEntity::find()
            .filter(UserColumn::Status.eq(Status::Enabled))
            .count(&*self.db)
            .await? as i64)
    }
}
