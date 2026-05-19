use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Statement,
};
use uuid::Uuid;

use crate::domain::entities::log_entry::{LogContent, LogEntry};
use crate::domain::repositories::log_repository::{LogQuery, LogRepository};
use crate::infrastructure::persistence::entities::log_content::{
    ActiveModel as ContentActiveModel, Entity as ContentEntity,
};
use crate::infrastructure::persistence::entities::log_metadata::{
    ActiveModel, Column, Entity,
};
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;

pub struct SeaOrmLogRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmLogRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        SeaOrmLogRepository { db }
    }
}

#[async_trait]
impl LogRepository for SeaOrmLogRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<LogEntry>, AppError> {
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

    async fn find_by_session_id(&self, session_id: &str) -> Result<Vec<LogEntry>, AppError> {
        let db = &*self.db;
        let models = Entity::find()
            .filter(Column::SessionId.eq(session_id))
            .order_by_asc(Column::Timestamp)
            .all(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        models
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<LogEntry>, AppError>>()
    }

    async fn find_all_paginated(
        &self,
        page: u64,
        page_size: u64,
        filter: &LogQuery,
    ) -> Result<PaginatedResult<LogEntry>, AppError> {
        let db = &*self.db;
        let page = page.max(1);
        let page_size = page_size.min(100);
        let utc_offset = FixedOffset::east_opt(0).expect("UTC 偏移");

        let mut select = Entity::find().order_by_desc(Column::Timestamp);

        if let Some(ref session_id) = filter.session_id {
            select = select.filter(Column::SessionId.eq(session_id));
        }
        if let Some(user_id) = &filter.user_id {
            select = select.filter(Column::UserId.eq(*user_id));
        }
        if let Some(access_point_id) = &filter.access_point_id {
            select = select.filter(Column::AccessPointId.eq(*access_point_id));
        }
        if let Some(start_time) = &filter.start_time {
            select = select.filter(Column::Timestamp.gte(start_time.with_timezone(&utc_offset)));
        }
        if let Some(end_time) = &filter.end_time {
            select = select.filter(Column::Timestamp.lte(end_time.with_timezone(&utc_offset)));
        }
        if let Some(status_code) = &filter.status_code {
            select = select.filter(Column::StatusCode.eq(*status_code));
        }

        let paginator = select.paginate(db, page_size);

        let total = paginator
            .num_items()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let models = paginator
            .fetch_page(page - 1)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let items = models
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<LogEntry>, AppError>>()?;

        Ok(PaginatedResult {
            items,
            total,
            page,
            page_size,
        })
    }

    async fn save(&self, entry: &LogEntry) -> Result<LogEntry, AppError> {
        let db = &*self.db;
        let exists = Entity::find_by_id(entry.id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .is_some();

        let active_model: ActiveModel = entry.clone().into();

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

        Entity::find_by_id(entry.id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .map(|m| m.try_into())
            .ok_or_else(|| AppError::Internal("保存后无法查询到 LogEntry".to_string()))?
    }

    async fn save_content(&self, content: &LogContent) -> Result<(), AppError> {
        let db = &*self.db;
        let active_model: ContentActiveModel = content.clone().into();
        ContentEntity::insert(active_model)
            .exec(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    async fn find_content_by_log_id(&self, log_id: Uuid) -> Result<Option<LogContent>, AppError> {
        let db = &*self.db;
        let model = ContentEntity::find_by_id(log_id)
            .one(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match model {
            Some(m) => Ok(Some(m.try_into()?)),
            None => Ok(None),
        }
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let db = &*self.db;
        // 先删除关联的 log_content（如果有）
        let _ = ContentEntity::delete_by_id(id).exec(db).await;
        // 再删除 log_metadata
        Entity::delete_by_id(id)
            .exec(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    // ─── 统计方法 ───

    async fn count_total(&self) -> Result<u64, AppError> {
        let db = &*self.db;
        let count = Entity::find()
            .count(db)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(count)
    }

    async fn count_by_date_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<(NaiveDate, u64)>, AppError> {
        let db = &*self.db;
        let sql = r#"
            SELECT DATE(timestamp)::TEXT AS day, COUNT(*)::BIGINT AS cnt
            FROM log_metadata
            WHERE timestamp >= $1::timestamptz AND timestamp < $2::timestamptz
            GROUP BY day
            ORDER BY day
        "#;

        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [start.to_rfc3339().into(), end.to_rfc3339().into()],
        );

        let results = db
            .query_all(stmt)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut data = Vec::new();
        for row in &results {
            let day_str: String = row
                .try_get_by_index(0)
                .map_err(|e| AppError::Database(e.to_string()))?;
            let day = NaiveDate::parse_from_str(&day_str, "%Y-%m-%d")
                .map_err(|e| AppError::Internal(format!("日期解析失败: {}", e)))?;
            let count: i64 = row
                .try_get_by_index(1)
                .map_err(|e| AppError::Database(e.to_string()))?;
            data.push((day, count as u64));
        }

        Ok(data)
    }

    async fn top_access_points(&self, limit: u64) -> Result<Vec<(Uuid, u64)>, AppError> {
        let db = &*self.db;
        let sql = r#"
            SELECT access_point_id, COUNT(*)::BIGINT AS cnt
            FROM log_metadata
            WHERE access_point_id IS NOT NULL
            GROUP BY access_point_id
            ORDER BY cnt DESC
            LIMIT $1
        "#;

        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [(limit as i64).into()],
        );

        let results = db
            .query_all(stmt)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut data = Vec::new();
        for row in &results {
            let id: Uuid = row
                .try_get_by_index(0)
                .map_err(|e| AppError::Database(e.to_string()))?;
            let count: i64 = row
                .try_get_by_index(1)
                .map_err(|e| AppError::Database(e.to_string()))?;
            data.push((id, count as u64));
        }

        Ok(data)
    }

    async fn top_models(&self, limit: u64) -> Result<Vec<(String, u64)>, AppError> {
        let db = &*self.db;
        let sql = r#"
            SELECT model_original, COUNT(*)::BIGINT AS cnt
            FROM log_metadata
            WHERE model_original IS NOT NULL
            GROUP BY model_original
            ORDER BY cnt DESC
            LIMIT $1
        "#;

        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [(limit as i64).into()],
        );

        let results = db
            .query_all(stmt)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut data = Vec::new();
        for row in &results {
            let model: String = row
                .try_get_by_index(0)
                .map_err(|e| AppError::Database(e.to_string()))?;
            let count: i64 = row
                .try_get_by_index(1)
                .map_err(|e| AppError::Database(e.to_string()))?;
            data.push((model, count as u64));
        }

        Ok(data)
    }

    async fn count_active_access_points(&self) -> Result<u64, AppError> {
        let db = &*self.db;
        let sql = r#"
            SELECT COUNT(DISTINCT access_point_id)::BIGINT AS cnt
            FROM log_metadata
            WHERE access_point_id IS NOT NULL
        "#;

        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, sql, []);

        let results = db
            .query_all(stmt)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let count: i64 = results
            .first()
            .ok_or_else(|| AppError::Internal("查询结果为空".to_string()))?
            .try_get_by_index(0)
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u64)
    }
}
