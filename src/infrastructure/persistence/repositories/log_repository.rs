//! 日志 Repository 实现（基础设施层）
//!
//! 实现了 `LogRepository` trait 的所有方法，包括：
//! - 基础 CRUD（metadata、content、词元用量）
//! - 分页查询（含 SQL 动态拼接和全文检索）
//! - 会话聚合查询
//! - Dashboard 聚合查询（KPI / sparkline / Top N 排行）

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Statement,
};
use uuid::Uuid;

use crate::domain::log::content::{ActiveModel as ContentActiveModel, Entity as ContentEntity};
use crate::domain::log::metadata::{ActiveModel, Column, Entity};
use crate::domain::log::{
    DashboardWindow, HeatmapCell, KpiAggregate, LogMetadataWithTokenSummary, LogQuery,
    LogRepository, QualityMetrics, SessionQuery, SessionSummaryData, SparklineBucket,
    TopAccessPointRow, TopModelRow, UsageTrendBucket,
};
use crate::domain::log::{LogContent, LogMetadata, LogTokenUsage};
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
    async fn find_by_id(&self, id: Uuid) -> Result<Option<LogMetadata>, AppError> {
        let db = &*self.db;
        let model = Entity::find_by_id(id).one(db).await?;

        match model {
            Some(m) => Ok(Some(m)),
            None => Ok(None),
        }
    }

    async fn find_by_session_id(&self, session_id: &str) -> Result<Vec<LogMetadata>, AppError> {
        let db = &*self.db;
        let models = Entity::find()
            .filter(Column::SessionId.eq(session_id))
            .order_by_asc(Column::Timestamp)
            .all(db)
            .await?;

        Ok(models)
    }

    async fn find_all_paginated(
        &self,
        page: u64,
        page_size: u64,
        filter: &LogQuery,
    ) -> Result<PaginatedResult<LogMetadata>, AppError> {
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
        if let Some(provider_id) = &filter.provider_id {
            select = select.filter(Column::ProviderId.eq(*provider_id));
        }
        if let Some(account_id) = &filter.account_id {
            select = select.filter(Column::AccountId.eq(*account_id));
        }
        if let Some(is_interrupted) = &filter.is_interrupted {
            select = select.filter(Column::IsInterrupted.eq(*is_interrupted));
        }

        let paginator = select.paginate(db, page_size);

        let total = paginator.num_items().await?;

        let models = paginator.fetch_page(page - 1).await?;

        let items = models;

        Ok(PaginatedResult {
            items,
            total,
            page,
            page_size,
        })
    }

    async fn save(&self, entry: &LogMetadata) -> Result<LogMetadata, AppError> {
        let db = &*self.db;
        let exists = Entity::find_by_id(entry.id).one(db).await?.is_some();

        let active_model: ActiveModel = entry.clone().into();

        if exists {
            let active_model = active_model.reset_all();
            Entity::update(active_model).exec(db).await?;
        } else {
            Entity::insert(active_model).exec(db).await?;
        }

        let result = Entity::find_by_id(entry.id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Internal("保存后无法查询到 LogMetadata".to_string()))?;
        Ok(result)
    }

    async fn save_content(&self, content: &LogContent) -> Result<(), AppError> {
        let db = &*self.db;
        let active_model: ContentActiveModel = content.clone().into();
        ContentEntity::insert(active_model).exec(db).await?;
        Ok(())
    }

    async fn find_content_by_log_id(&self, log_id: Uuid) -> Result<Option<LogContent>, AppError> {
        use crate::domain::log::content::Column;
        let db = &*self.db;
        let model = ContentEntity::find()
            .filter(Column::LogId.eq(log_id))
            .one(db)
            .await?;

        match model {
            Some(m) => Ok(Some(m)),
            None => Ok(None),
        }
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        use crate::domain::log::content::Column;
        let db = &*self.db;
        // 先删除关联的 log_content（如果有）
        let _ = ContentEntity::delete_many()
            .filter(Column::LogId.eq(id))
            .exec(db)
            .await;
        // 再删除 log_metadata
        Entity::delete_by_id(id).exec(db).await?;
        Ok(())
    }

    // ─── 新查询方法 ───

    async fn find_all_paginated_with_token_summary(
        &self,
        page: u64,
        page_size: u64,
        filter: &LogQuery,
    ) -> Result<PaginatedResult<LogMetadataWithTokenSummary>, AppError> {
        let db = &*self.db;
        let page = page.max(1);
        let page_size = page_size.min(100);
        let offset = (page - 1) * page_size;
        let utc_offset = FixedOffset::east_opt(0).expect("UTC 偏移");

        // 构建 WHERE 条件参数
        let mut param_index = 1u32;

        // 先查询总数
        let mut count_sql =
            "SELECT COUNT(*)::BIGINT AS cnt FROM log_metadata lm WHERE 1=1".to_string();
        let mut count_params: Vec<sea_orm::Value> = Vec::new();

        // 数据查询 SQL
        let mut data_sql = r#"
            SELECT
                lm.id, lm.timestamp, lm.session_id, lm.user_id,
                lm.access_point_id, lm.provider_id, lm.account_id,
                lm.model_original, lm.model_mapped,
                lm.status_code, lm.duration_ms, lm.error_message,
                lm.client_user_agent,
                lm.conversation_source,
                lm.agent_id,
                lm.has_error, lm.raw_content_available, lm.is_interrupted,
                lm.client_version,
                lm.api_type, lm.client_type,
                ltu.input_tokens, ltu.output_tokens,
                ltu.cache_creation_input_tokens, ltu.cache_read_input_tokens,
                ltu.thinking_tokens, ltu.total_tokens
            FROM log_metadata lm
            LEFT JOIN log_token_usage ltu ON ltu.log_id = lm.id
            WHERE 1=1
        "#
        .to_string();
        let mut data_params: Vec<sea_orm::Value> = Vec::new();

        if let Some(ref session_id) = filter.session_id {
            let p = format!("${}", param_index);
            param_index += 1;
            let cond = format!(" AND lm.session_id = {}", p);
            let val: sea_orm::Value = session_id.clone().into();
            count_sql.push_str(&cond);
            data_sql.push_str(&cond);
            count_params.push(val.clone());
            data_params.push(val);
        }
        if let Some(user_id) = &filter.user_id {
            let p = format!("${}", param_index);
            param_index += 1;
            let cond = format!(" AND lm.user_id = {}", p);
            let val: sea_orm::Value = (*user_id).into();
            count_sql.push_str(&cond);
            data_sql.push_str(&cond);
            count_params.push(val.clone());
            data_params.push(val);
        }
        if let Some(access_point_id) = &filter.access_point_id {
            let p = format!("${}", param_index);
            param_index += 1;
            let cond = format!(" AND lm.access_point_id = {}", p);
            let val: sea_orm::Value = (*access_point_id).into();
            count_sql.push_str(&cond);
            data_sql.push_str(&cond);
            count_params.push(val.clone());
            data_params.push(val);
        }
        if let Some(start_time) = &filter.start_time {
            let p = format!("${}", param_index);
            param_index += 1;
            let cond = format!(" AND lm.timestamp >= {}::timestamptz", p);
            count_sql.push_str(&cond);
            data_sql.push_str(&cond);
            count_params.push(start_time.with_timezone(&utc_offset).into());
            data_params.push(start_time.with_timezone(&utc_offset).into());
        }
        if let Some(end_time) = &filter.end_time {
            let p = format!("${}", param_index);
            param_index += 1;
            let cond = format!(" AND lm.timestamp <= {}::timestamptz", p);
            count_sql.push_str(&cond);
            data_sql.push_str(&cond);
            count_params.push(end_time.with_timezone(&utc_offset).into());
            data_params.push(end_time.with_timezone(&utc_offset).into());
        }
        if let Some(status_code) = &filter.status_code {
            let p = format!("${}", param_index);
            param_index += 1;
            let cond = format!(" AND lm.status_code = {}", p);
            let val: sea_orm::Value = (*status_code).into();
            count_sql.push_str(&cond);
            data_sql.push_str(&cond);
            count_params.push(val.clone());
            data_params.push(val);
        }
        if let Some(provider_id) = &filter.provider_id {
            let p = format!("${}", param_index);
            param_index += 1;
            let cond = format!(" AND lm.provider_id = {}", p);
            let val: sea_orm::Value = (*provider_id).into();
            count_sql.push_str(&cond);
            data_sql.push_str(&cond);
            count_params.push(val.clone());
            data_params.push(val);
        }
        if let Some(account_id) = &filter.account_id {
            let p = format!("${}", param_index);
            param_index += 1;
            let cond = format!(" AND lm.account_id = {}", p);
            let val: sea_orm::Value = (*account_id).into();
            count_sql.push_str(&cond);
            data_sql.push_str(&cond);
            count_params.push(val.clone());
            data_params.push(val);
        }
        if let Some(is_interrupted) = &filter.is_interrupted {
            let p = format!("${}", param_index);
            param_index += 1;
            let cond = format!(" AND lm.is_interrupted = {}", p);
            let val: sea_orm::Value = (*is_interrupted).into();
            count_sql.push_str(&cond);
            data_sql.push_str(&cond);
            count_params.push(val.clone());
            data_params.push(val);
        }

        // 添加排序和分页
        let limit_placeholder = format!("${}", param_index);
        param_index += 1;
        let offset_placeholder = format!("${}", param_index);

        data_sql.push_str(&format!(
            " ORDER BY lm.timestamp DESC LIMIT {} OFFSET {}",
            limit_placeholder, offset_placeholder
        ));
        data_params.push((page_size as i64).into());
        data_params.push((offset as i64).into());

        // 执行 count 查询
        let count_stmt =
            Statement::from_sql_and_values(DbBackend::Postgres, &count_sql, count_params);
        let count_result = db
            .query_one_raw(count_stmt)
            .await?
            .ok_or_else(|| AppError::Internal("计数查询结果为空".to_string()))?;
        let total: i64 = count_result.try_get_by_index(0)?;
        let total = total as u64;

        if total == 0 {
            return Ok(PaginatedResult {
                items: Vec::new(),
                total: 0,
                page,
                page_size,
            });
        }

        // 执行数据查询
        let data_stmt = Statement::from_sql_and_values(DbBackend::Postgres, &data_sql, data_params);
        let results = db.query_all_raw(data_stmt).await?;

        let items: Vec<LogMetadataWithTokenSummary> = results
            .iter()
            .map(|row| {
                let id: Uuid = row.try_get_by_index::<Uuid>(0)?;

                let timestamp_col: chrono::DateTime<FixedOffset> = row.try_get_by_index(1)?;

                Ok(LogMetadataWithTokenSummary {
                    entry: LogMetadata {
                        id,
                        timestamp: timestamp_col,
                        session_id: row.try_get_by_index::<String>(2)?,
                        user_id: row.try_get_by_index::<Option<Uuid>>(3)?,
                        access_point_id: row.try_get_by_index::<Option<Uuid>>(4)?,
                        provider_id: row.try_get_by_index::<Option<Uuid>>(5)?,
                        account_id: row.try_get_by_index::<Option<Uuid>>(6)?,
                        model_original: row.try_get_by_index::<Option<String>>(7)?,
                        model_mapped: row.try_get_by_index::<Option<String>>(8)?,
                        status_code: row.try_get_by_index::<Option<i16>>(9)?,
                        duration_ms: row.try_get_by_index::<Option<i32>>(10)?,
                        error_message: row.try_get_by_index::<Option<String>>(11)?,
                        client_user_agent: row.try_get_by_index::<Option<String>>(12)?,
                        conversation_source: row.try_get_by_index::<String>(13)?,
                        agent_id: row.try_get_by_index::<Option<String>>(14)?,
                        has_error: row.try_get_by_index::<bool>(15)?,
                        raw_content_available: row.try_get_by_index::<bool>(16)?,
                        is_interrupted: row.try_get_by_index::<bool>(17)?,
                        client_version: row.try_get_by_index::<Option<String>>(18)?,
                        api_type: row.try_get_by_index::<String>(19)?,
                        client_type: row.try_get_by_index::<String>(20)?,
                    },
                    input_tokens: row.try_get_by_index::<Option<i32>>(21)?,
                    output_tokens: row.try_get_by_index::<Option<i32>>(22)?,
                    cache_creation_input_tokens: row.try_get_by_index::<Option<i32>>(23)?,
                    cache_read_input_tokens: row.try_get_by_index::<Option<i32>>(24)?,
                    thinking_tokens: row.try_get_by_index::<Option<i32>>(25)?,
                    total_tokens: row.try_get_by_index::<Option<i32>>(26)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(PaginatedResult {
            items,
            total,
            page,
            page_size,
        })
    }

    async fn find_sessions_paginated(
        &self,
        page: u64,
        page_size: u64,
        filter: &SessionQuery,
    ) -> Result<PaginatedResult<SessionSummaryData>, AppError> {
        let db = &*self.db;
        let page = page.max(1);
        let page_size = page_size.min(100);
        let offset = (page - 1) * page_size;
        let mut param_index = 1u32;

        // 构建 SQL 参数
        let mut count_params: Vec<sea_orm::Value> = Vec::new();
        let mut data_params: Vec<sea_orm::Value> = Vec::new();

        let mut where_clauses = Vec::new();

        if let Some(ref session_id) = filter.session_id {
            let p = format!("${}", param_index);
            param_index += 1;
            where_clauses.push(format!("lm.session_id = {}", p));
            let val: sea_orm::Value = session_id.clone().into();
            count_params.push(val.clone());
            data_params.push(val);
        }
        if let Some(user_id) = &filter.user_id {
            let p = format!("${}", param_index);
            param_index += 1;
            where_clauses.push(format!("lm.user_id = {}", p));
            let val: sea_orm::Value = (*user_id).into();
            count_params.push(val.clone());
            data_params.push(val);
        }
        if let Some(access_point_id) = &filter.access_point_id {
            let p = format!("${}", param_index);
            param_index += 1;
            where_clauses.push(format!("lm.access_point_id = {}", p));
            let val: sea_orm::Value = (*access_point_id).into();
            count_params.push(val.clone());
            data_params.push(val);
        }
        if let Some(start_time) = &filter.start_time {
            let p = format!("${}", param_index);
            param_index += 1;
            where_clauses.push(format!("lm.timestamp >= {}::timestamptz", p));
            let val: sea_orm::Value = start_time.to_rfc3339().into();
            count_params.push(val.clone());
            data_params.push(val);
        }
        if let Some(end_time) = &filter.end_time {
            let p = format!("${}", param_index);
            param_index += 1;
            where_clauses.push(format!("lm.timestamp <= {}::timestamptz", p));
            let val: sea_orm::Value = end_time.to_rfc3339().into();
            count_params.push(val.clone());
            data_params.push(val);
        }
        if let Some(status_code) = &filter.status_code {
            let p = format!("${}", param_index);
            param_index += 1;
            where_clauses.push(format!("lm.status_code = {}", p));
            let val: sea_orm::Value = (*status_code).into();
            count_params.push(val.clone());
            data_params.push(val);
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        // 总数查询
        let count_sql = format!(
            "SELECT COUNT(*)::BIGINT FROM (SELECT 1 FROM log_metadata lm {} GROUP BY lm.session_id) sub",
            where_sql
        );
        let count_stmt =
            Statement::from_sql_and_values(DbBackend::Postgres, &count_sql, count_params);
        let count_result = db
            .query_one_raw(count_stmt)
            .await?
            .ok_or_else(|| AppError::Internal("会话计数查询结果为空".to_string()))?;
        let total: i64 = count_result.try_get_by_index(0)?;
        let total = total as u64;

        if total == 0 {
            return Ok(PaginatedResult {
                items: Vec::new(),
                total: 0,
                page,
                page_size,
            });
        }

        // 数据查询
        let limit_p = format!("${}", param_index);
        param_index += 1;
        let offset_p = format!("${}", param_index);
        data_params.push((page_size as i64).into());
        data_params.push((offset as i64).into());

        let data_sql = format!(
            r#"
            SELECT
                lm.session_id,
                MIN(lm.user_id::text)::uuid as user_id,
                MIN(lm.access_point_id::text)::uuid as access_point_id,
                MIN(lm.timestamp) as start_time,
                CAST(COUNT(*) AS BIGINT) as request_count,
                COALESCE(SUM(ltu.input_tokens), 0)::BIGINT as total_input_tokens,
                COALESCE(SUM(ltu.output_tokens), 0)::BIGINT as total_output_tokens,
                COALESCE(SUM(ltu.cache_creation_input_tokens), 0)::BIGINT as total_cache_creation_input_tokens,
                COALESCE(SUM(ltu.cache_read_input_tokens), 0)::BIGINT as total_cache_read_input_tokens,
                COALESCE(SUM(ltu.thinking_tokens), 0)::BIGINT as total_thinking_tokens,
                COALESCE(SUM(ltu.total_tokens), 0)::BIGINT as total_tokens
            FROM log_metadata lm
            LEFT JOIN log_token_usage ltu ON ltu.log_id = lm.id
            {}
            GROUP BY lm.session_id
            ORDER BY start_time DESC
            LIMIT {} OFFSET {}
            "#,
            where_sql, limit_p, offset_p
        );

        let data_stmt = Statement::from_sql_and_values(DbBackend::Postgres, &data_sql, data_params);
        let results = db.query_all_raw(data_stmt).await?;

        let items: Vec<SessionSummaryData> = results
            .iter()
            .map(|row| {
                let start_time_col: chrono::DateTime<FixedOffset> = row.try_get_by_index(3)?;

                Ok(SessionSummaryData {
                    session_id: row.try_get_by_index::<String>(0)?,
                    user_id: row.try_get_by_index::<Option<Uuid>>(1)?,
                    access_point_id: row.try_get_by_index::<Option<Uuid>>(2)?,
                    start_time: start_time_col.to_utc(),
                    request_count: row.try_get_by_index::<i64>(4)?,
                    total_input_tokens: row.try_get_by_index::<i64>(5)?,
                    total_output_tokens: row.try_get_by_index::<i64>(6)?,
                    total_cache_creation_input_tokens: row.try_get_by_index::<i64>(7)?,
                    total_cache_read_input_tokens: row.try_get_by_index::<i64>(8)?,
                    total_thinking_tokens: row.try_get_by_index::<i64>(9)?,
                    total_tokens: row.try_get_by_index::<i64>(10)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(PaginatedResult {
            items,
            total,
            page,
            page_size,
        })
    }

    async fn find_log_detail_full(
        &self,
        id: Uuid,
    ) -> Result<Option<(LogMetadata, LogContent, Option<LogTokenUsage>)>, AppError> {
        let db = &*self.db;

        let sql = r#"
            SELECT
                lm.id, lm.timestamp, lm.session_id, lm.user_id,
                lm.access_point_id, lm.provider_id, lm.account_id,
                lm.model_original, lm.model_mapped,
                lm.status_code, lm.duration_ms, lm.error_message,
                lm.client_user_agent,
                lm.conversation_source,
                lm.agent_id,
                lm.has_error, lm.raw_content_available, lm.is_interrupted,
                lm.client_version,
                lm.api_type, lm.client_type,
                lc.request_headers, lc.request_body, lc.response_body, lc.response_headers,
                ltu.id as usage_id, ltu.log_id as usage_log_id,
                ltu.input_tokens, ltu.output_tokens,
                ltu.cache_creation_input_tokens, ltu.cache_read_input_tokens,
                ltu.thinking_tokens, ltu.total_tokens, ltu.raw_usage,
                ltu.server_tool_usage, ltu.cache_creation,
                ltu.client_type as usage_client_type,
                ltu.session_id as usage_session_id, ltu.timestamp as usage_timestamp,
                ltu.user_id as usage_user_id, ltu.access_point_id as usage_access_point_id,
                ltu.provider_id as usage_provider_id, ltu.account_id as usage_account_id,
                ltu.model_original as usage_model_original,
                ltu.model_mapped as usage_model_mapped,
                ltu.conversation_source as usage_conversation_source,
                ltu.agent_id as usage_agent_id, ltu.agent_type as usage_agent_type,
                ltu.created_at as usage_created_at
            FROM log_metadata lm
            LEFT JOIN log_contents lc ON lc.log_id = lm.id
            LEFT JOIN log_token_usage ltu ON ltu.log_id = lm.id
            WHERE lm.id = $1::uuid
        "#;

        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, sql, [id.into()]);
        let result = db.query_one_raw(stmt).await?;

        match result {
            Some(row) => {
                let timestamp_col: chrono::DateTime<FixedOffset> = row.try_get_by_index(1)?;

                let entry = LogMetadata {
                    id: row.try_get_by_index::<Uuid>(0)?,
                    timestamp: timestamp_col,
                    session_id: row.try_get_by_index::<String>(2)?,
                    user_id: row.try_get_by_index::<Option<Uuid>>(3)?,
                    access_point_id: row.try_get_by_index::<Option<Uuid>>(4)?,
                    provider_id: row.try_get_by_index::<Option<Uuid>>(5)?,
                    account_id: row.try_get_by_index::<Option<Uuid>>(6)?,
                    model_original: row.try_get_by_index::<Option<String>>(7)?,
                    model_mapped: row.try_get_by_index::<Option<String>>(8)?,
                    status_code: row.try_get_by_index::<Option<i16>>(9)?,
                    duration_ms: row.try_get_by_index::<Option<i32>>(10)?,
                    error_message: row.try_get_by_index::<Option<String>>(11)?,
                    client_user_agent: row.try_get_by_index::<Option<String>>(12)?,
                    conversation_source: row.try_get_by_index::<String>(13)?,
                    agent_id: row.try_get_by_index::<Option<String>>(14)?,
                    has_error: row.try_get_by_index::<bool>(15)?,
                    raw_content_available: row.try_get_by_index::<bool>(16)?,
                    is_interrupted: row.try_get_by_index::<bool>(17)?,
                    client_version: row.try_get_by_index::<Option<String>>(18)?,
                    api_type: row.try_get_by_index::<String>(19)?,
                    client_type: row.try_get_by_index::<String>(20)?,
                };

                let content = LogContent {
                    log_id: entry.id,
                    timestamp: entry.timestamp,
                    request_headers: Some(
                        row.try_get_by_index::<Option<serde_json::Value>>(21)?
                            .unwrap_or(serde_json::Value::Null),
                    ),
                    request_body: Some(
                        row.try_get_by_index::<Option<serde_json::Value>>(22)?
                            .unwrap_or(serde_json::Value::Null),
                    ),
                    response_body: Some(
                        row.try_get_by_index::<Option<String>>(23)?
                            .unwrap_or_default(),
                    ),
                    response_headers: row.try_get_by_index::<Option<serde_json::Value>>(24)?,
                };

                // 检查是否有词元用量（ltu.id 不为 NULL）
                let usage_id: Option<Uuid> = row.try_get_by_index::<Option<Uuid>>(25)?;

                let usage = if let Some(uid) = usage_id {
                    let usage_ts_col: chrono::DateTime<FixedOffset> = row.try_get_by_index(38)?;
                    let usage_created_col: chrono::DateTime<FixedOffset> =
                        row.try_get_by_index(48)?;

                    Some(LogTokenUsage {
                        id: uid,
                        log_id: row
                            .try_get_by_index::<Option<Uuid>>(26)?
                            .unwrap_or(entry.id),
                        input_tokens: row.try_get_by_index::<i32>(27)?,
                        output_tokens: row.try_get_by_index::<i32>(28)?,
                        cache_creation_input_tokens: row.try_get_by_index::<i32>(29)?,
                        cache_read_input_tokens: row.try_get_by_index::<i32>(30)?,
                        thinking_tokens: row.try_get_by_index::<i32>(31)?,
                        total_tokens: row.try_get_by_index::<i32>(32)?,
                        raw_usage: row.try_get_by_index::<Option<serde_json::Value>>(33)?,
                        server_tool_usage: row.try_get_by_index::<Option<serde_json::Value>>(34)?,
                        cache_creation: row.try_get_by_index::<Option<serde_json::Value>>(35)?,
                        client_type: row
                            .try_get_by_index::<Option<String>>(36)?
                            .unwrap_or_default(),
                        session_id: row
                            .try_get_by_index::<Option<String>>(37)?
                            .unwrap_or_default(),
                        timestamp: usage_ts_col,
                        user_id: row.try_get_by_index::<Option<Uuid>>(39)?,
                        access_point_id: row.try_get_by_index::<Option<Uuid>>(40)?,
                        provider_id: row.try_get_by_index::<Option<Uuid>>(41)?,
                        account_id: row.try_get_by_index::<Option<Uuid>>(42)?,
                        model_original: row.try_get_by_index::<Option<String>>(43)?,
                        model_mapped: row.try_get_by_index::<Option<String>>(44)?,
                        conversation_source: row.try_get_by_index::<Option<String>>(45)?,
                        agent_id: row.try_get_by_index::<Option<String>>(46)?,
                        agent_type: row.try_get_by_index::<Option<String>>(47)?,
                        created_at: usage_created_col,
                    })
                } else {
                    None
                };

                Ok(Some((entry, content, usage)))
            }
            None => Ok(None),
        }
    }

    // ─── Dashboard 聚合查询（个人视角，所有方法均按 user_id 过滤）───

    /// KPI 聚合：单次 SQL 返回个人视角的请求数与 6 类词元 SUM。
    ///
    /// 1. WHERE 子句先按 `user_id` 过滤，再按时间窗口剪枝（`log_metadata` 按月分区，
    ///    PostgreSQL 会自动剪掉不相关的子分区）。
    /// 2. `log_token_usage` 通过 `log_id` 与 `log_metadata` 一对一关联，LEFT JOIN
    ///    保证未写入词元用量的请求仍计入 `request_count`。
    /// 3. `total_input_side_tokens` 是输入方向词元总量（缓存命中率的分母），单独聚合可避免前端再算一次。
    #[tracing::instrument(
        skip(self),
        fields(user_id = %user_id, window.start = %window.start, window.end = %window.end)
    )]
    async fn aggregate_kpi(
        &self,
        user_id: Uuid,
        window: &DashboardWindow,
    ) -> Result<KpiAggregate, AppError> {
        let db = &*self.db;
        let sql = r#"
            SELECT
                COUNT(*)::BIGINT AS request_count,
                COUNT(DISTINCT lm.session_id)::BIGINT AS session_count,
                COALESCE(SUM(ltu.total_tokens), 0)::BIGINT AS total_tokens,
                COALESCE(SUM(ltu.input_tokens), 0)::BIGINT AS input_tokens,
                COALESCE(SUM(ltu.output_tokens), 0)::BIGINT AS output_tokens,
                COALESCE(SUM(ltu.cache_creation_input_tokens), 0)::BIGINT AS cache_creation_tokens,
                COALESCE(SUM(ltu.cache_read_input_tokens), 0)::BIGINT AS cache_read_tokens,
                COALESCE(SUM(ltu.thinking_tokens), 0)::BIGINT AS thinking_tokens,
                COALESCE(SUM(ltu.input_tokens + ltu.cache_creation_input_tokens + ltu.cache_read_input_tokens), 0)::BIGINT AS total_input_side_tokens
            FROM log_metadata lm
            LEFT JOIN log_token_usage ltu ON ltu.log_id = lm.id
            WHERE lm.user_id = $1::uuid
              AND lm.timestamp >= $2::timestamptz
              AND lm.timestamp < $3::timestamptz
        "#;

        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [user_id.into(), window.start.into(), window.end.into()],
        );

        let row = db
            .query_one_raw(stmt)
            .await?
            .ok_or_else(|| AppError::Internal("KPI 聚合查询无结果".to_string()))?;

        Ok(KpiAggregate {
            request_count: row.try_get_by_index::<i64>(0)?,
            session_count: row.try_get_by_index::<i64>(1)?,
            total_tokens: row.try_get_by_index::<i64>(2)?,
            input_tokens: row.try_get_by_index::<i64>(3)?,
            output_tokens: row.try_get_by_index::<i64>(4)?,
            cache_creation_tokens: row.try_get_by_index::<i64>(5)?,
            cache_read_tokens: row.try_get_by_index::<i64>(6)?,
            thinking_tokens: row.try_get_by_index::<i64>(7)?,
            total_input_side_tokens: row.try_get_by_index::<i64>(8)?,
        })
    }

    /// Sparkline 聚合：按 hour 或 day 分桶，用 `generate_series` 补齐空桶。
    ///
    /// `bucket_count == 24` 时按小时聚合（用于"今日"视图），否则按天聚合。
    /// 桶区间为 `[date_trunc($1), date_trunc($2 - 1 epoch))`，与 `window` 的闭右开语义一致。
    /// SQL 内 WHERE 子句额外加 `AND lm.user_id = $3::uuid`，按用户视角过滤。
    #[tracing::instrument(
        skip(self),
        fields(
            user_id = %user_id,
            window.start = %window.start,
            window.end = %window.end,
            bucket_count = bucket_count
        )
    )]
    async fn aggregate_sparkline(
        &self,
        user_id: Uuid,
        window: &DashboardWindow,
        bucket_count: u32,
    ) -> Result<Vec<SparklineBucket>, AppError> {
        let db = &*self.db;

        // 桶粒度由 bucket_count 决定：24 → 小时桶，其余 → 日桶
        let unit = if bucket_count == 24 { "hour" } else { "day" };

        let sql = format!(
            r#"
            WITH series AS (
                SELECT generate_series(
                    date_trunc('{unit}', $1::timestamptz),
                    date_trunc('{unit}', $2::timestamptz - interval '1 second'),
                    interval '1 {unit}'
                ) AS bucket_start
            ), data AS (
                SELECT
                    date_trunc('{unit}', lm.timestamp) AS bucket_start,
                    COUNT(*)::BIGINT AS request_count,
                    COALESCE(SUM(ltu.total_tokens), 0)::BIGINT AS total_tokens
                FROM log_metadata lm
                LEFT JOIN log_token_usage ltu ON ltu.log_id = lm.id
                WHERE lm.timestamp >= $1::timestamptz
                  AND lm.timestamp < $2::timestamptz
                  AND lm.user_id = $3::uuid
                GROUP BY 1
            )
            SELECT
                s.bucket_start,
                COALESCE(d.request_count, 0)::BIGINT,
                COALESCE(d.total_tokens, 0)::BIGINT
            FROM series s
            LEFT JOIN data d USING (bucket_start)
            ORDER BY s.bucket_start
            "#
        );

        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            &sql,
            [window.start.into(), window.end.into(), user_id.into()],
        );

        let results = db.query_all_raw(stmt).await?;

        let buckets = results
            .iter()
            .map(|row| {
                let bucket_start: DateTime<FixedOffset> = row.try_get_by_index(0)?;
                Ok(SparklineBucket {
                    bucket_start: bucket_start.with_timezone(&Utc),
                    request_count: row.try_get_by_index::<i64>(1)?,
                    total_tokens: row.try_get_by_index::<i64>(2)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(buckets)
    }

    /// 用量趋势聚合：按日补齐请求数与词元分项。
    ///
    /// 1. `usage_by_log` 先按 `log_id` 聚合词元，避免未来 token 表扩展为多行后 JOIN 放大。
    /// 2. `data` 仅按当前用户与闭右开窗口过滤，保证个人视角隔离。
    /// 3. `series` 用日级 `generate_series` 补齐空桶，前端无需再补缺口。
    #[tracing::instrument(
        skip(self),
        fields(user_id = %user_id, window.start = %window.start, window.end = %window.end)
    )]
    async fn usage_trends_for_user(
        &self,
        user_id: Uuid,
        window: &DashboardWindow,
    ) -> Result<Vec<UsageTrendBucket>, AppError> {
        let db = &*self.db;
        let sql = r#"
            WITH series AS (
                SELECT generate_series(
                    date_trunc('day', $2::timestamptz),
                    date_trunc('day', $3::timestamptz - interval '1 second'),
                    interval '1 day'
                ) AS bucket_start
            ), usage_by_log AS (
                SELECT
                    log_id,
                    COALESCE(SUM(total_tokens), 0)::BIGINT AS total_tokens,
                    COALESCE(SUM(input_tokens), 0)::BIGINT AS input_tokens,
                    COALESCE(SUM(output_tokens), 0)::BIGINT AS output_tokens,
                    COALESCE(SUM(cache_creation_input_tokens), 0)::BIGINT AS cache_creation_tokens,
                    COALESCE(SUM(cache_read_input_tokens), 0)::BIGINT AS cache_read_tokens,
                    COALESCE(SUM(thinking_tokens), 0)::BIGINT AS thinking_tokens
                FROM log_token_usage
                GROUP BY log_id
            ), data AS (
                SELECT
                    date_trunc('day', lm.timestamp) AS bucket_start,
                    COUNT(*)::BIGINT AS request_count,
                    COUNT(DISTINCT lm.session_id)::BIGINT AS session_count,
                    COALESCE(SUM(ubl.total_tokens), 0)::BIGINT AS total_tokens,
                    COALESCE(SUM(ubl.input_tokens), 0)::BIGINT AS input_tokens,
                    COALESCE(SUM(ubl.output_tokens), 0)::BIGINT AS output_tokens,
                    COALESCE(SUM(ubl.cache_creation_tokens), 0)::BIGINT AS cache_creation_tokens,
                    COALESCE(SUM(ubl.cache_read_tokens), 0)::BIGINT AS cache_read_tokens,
                    COALESCE(SUM(ubl.thinking_tokens), 0)::BIGINT AS thinking_tokens
                FROM log_metadata lm
                LEFT JOIN usage_by_log ubl ON ubl.log_id = lm.id
                WHERE lm.user_id = $1::uuid
                  AND lm.timestamp >= $2::timestamptz
                  AND lm.timestamp < $3::timestamptz
                GROUP BY 1
            )
            SELECT
                s.bucket_start,
                COALESCE(d.request_count, 0)::BIGINT AS request_count,
                COALESCE(d.session_count, 0)::BIGINT AS session_count,
                COALESCE(d.total_tokens, 0)::BIGINT AS total_tokens,
                COALESCE(d.input_tokens, 0)::BIGINT AS input_tokens,
                COALESCE(d.output_tokens, 0)::BIGINT AS output_tokens,
                COALESCE(d.cache_creation_tokens, 0)::BIGINT AS cache_creation_tokens,
                COALESCE(d.cache_read_tokens, 0)::BIGINT AS cache_read_tokens,
                COALESCE(d.thinking_tokens, 0)::BIGINT AS thinking_tokens
            FROM series s
            LEFT JOIN data d USING (bucket_start)
            ORDER BY s.bucket_start
        "#;

        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [user_id.into(), window.start.into(), window.end.into()],
        );

        let results = db.query_all_raw(stmt).await?;
        let buckets = results
            .iter()
            .map(|row| {
                let bucket_start: DateTime<FixedOffset> = row.try_get_by_index(0)?;
                Ok(UsageTrendBucket {
                    bucket_start: bucket_start.with_timezone(&Utc),
                    request_count: row.try_get_by_index::<i64>(1)?,
                    session_count: row.try_get_by_index::<i64>(2)?,
                    total_tokens: row.try_get_by_index::<i64>(3)?,
                    input_tokens: row.try_get_by_index::<i64>(4)?,
                    output_tokens: row.try_get_by_index::<i64>(5)?,
                    cache_creation_tokens: row.try_get_by_index::<i64>(6)?,
                    cache_read_tokens: row.try_get_by_index::<i64>(7)?,
                    thinking_tokens: row.try_get_by_index::<i64>(8)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(buckets)
    }

    /// 用户日级 365 天词元热力图。
    ///
    /// 1. `series` CTE 在浏览器时区下补齐 365 个日桶（含今天）。
    /// 2. `data` CTE 按用户过滤后，将 `timestamp` 转到目标时区再按日截断分组。
    /// 3. 主查询用 `LEFT JOIN ... USING (day_local)` 把空日补 0，保证返回 365 行。
    ///
    /// 时区拼接：`timezone` 在 application 层已通过 `chrono_tz::Tz::from_str` 白名单
    /// 校验，无 SQL 注入风险；此处用 `format!` 直接拼接是因为 PostgreSQL 不允许
    /// `AT TIME ZONE` 接受参数占位符。
    #[tracing::instrument(skip(self), fields(user_id = %user_id, end = %end, timezone = timezone))]
    async fn user_daily_token_heatmap(
        &self,
        user_id: Uuid,
        end: DateTime<Utc>,
        timezone: &str,
    ) -> Result<Vec<HeatmapCell>, AppError> {
        let db = &*self.db;

        let sql = format!(
            r#"
            WITH series AS (
                SELECT generate_series(
                    date_trunc('day', ($1::timestamptz - interval '364 days') AT TIME ZONE '{tz}'),
                    date_trunc('day', $1::timestamptz AT TIME ZONE '{tz}'),
                    interval '1 day'
                ) AS day_local
            ), data AS (
                SELECT
                    date_trunc('day', lm.timestamp AT TIME ZONE '{tz}') AS day_local,
                    COUNT(*)::BIGINT AS request_count,
                    COALESCE(SUM(ltu.total_tokens), 0)::BIGINT AS total_tokens
                FROM log_metadata lm
                LEFT JOIN log_token_usage ltu ON ltu.log_id = lm.id
                WHERE lm.user_id = $2::uuid
                  AND lm.timestamp >= ($1::timestamptz - interval '365 days')
                  AND lm.timestamp <= $1::timestamptz
                GROUP BY 1
            )
            SELECT
                s.day_local::date AS day,
                COALESCE(d.request_count, 0)::BIGINT AS request_count,
                COALESCE(d.total_tokens, 0)::BIGINT AS total_tokens
            FROM series s
            LEFT JOIN data d USING (day_local)
            ORDER BY s.day_local
            "#,
            tz = timezone
        );

        let stmt =
            Statement::from_sql_and_values(DbBackend::Postgres, &sql, [end.into(), user_id.into()]);

        let results = db.query_all_raw(stmt).await?;

        let cells = results
            .iter()
            .map(|row| {
                Ok(HeatmapCell {
                    day: row.try_get_by_index::<NaiveDate>(0)?,
                    request_count: row.try_get_by_index::<i64>(1)?,
                    total_tokens: row.try_get_by_index::<i64>(2)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(cells)
    }

    /// 用户视角模型排行 Top N。
    ///
    /// 按 `model_mapped`（回落 `model_original`，再回落字符串 `(未知)`）分组聚合。
    /// `WHERE lm.user_id = $1` 按用户视角过滤，仅返回本人的模型使用情况。
    #[tracing::instrument(
        skip(self),
        fields(user_id = %user_id, window.start = %window.start, window.end = %window.end, limit = limit)
    )]
    async fn top_models_for_user(
        &self,
        user_id: Uuid,
        window: &DashboardWindow,
        limit: u32,
    ) -> Result<Vec<TopModelRow>, AppError> {
        let db = &*self.db;
        let sql = r#"
            SELECT
                COALESCE(lm.model_mapped, lm.model_original, '(未知)') AS model,
                COUNT(*)::BIGINT AS request_count,
                COALESCE(SUM(ltu.total_tokens), 0)::BIGINT AS total_tokens
            FROM log_metadata lm
            LEFT JOIN log_token_usage ltu ON ltu.log_id = lm.id
            WHERE lm.user_id = $1::uuid
              AND lm.timestamp >= $2::timestamptz
              AND lm.timestamp < $3::timestamptz
            GROUP BY 1
            ORDER BY request_count DESC
            LIMIT $4
        "#;

        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [
                user_id.into(),
                window.start.into(),
                window.end.into(),
                (limit as i64).into(),
            ],
        );

        let results = db.query_all_raw(stmt).await?;

        let rows = results
            .iter()
            .map(|row| {
                Ok(TopModelRow {
                    model: row.try_get_by_index::<String>(0)?,
                    request_count: row.try_get_by_index::<i64>(1)?,
                    total_tokens: row.try_get_by_index::<i64>(2)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(rows)
    }

    /// 用户视角接入点排行 Top N。
    ///
    /// 1. `WHERE lm.user_id = $1` 按用户视角过滤；`access_point_id IS NOT NULL`
    ///    剔除无接入点归属的请求。
    /// 2. `LEFT JOIN access_points` 容忍接入点删除：删除后 `name` / `short_code` 为 NULL，
    ///    前端降级展示为「已删除接入点」。
    /// 3. 排序按 `total_tokens DESC`，词元消耗大者优先。
    #[tracing::instrument(
        skip(self),
        fields(user_id = %user_id, window.start = %window.start, window.end = %window.end, limit = limit)
    )]
    async fn top_access_points_for_user(
        &self,
        user_id: Uuid,
        window: &DashboardWindow,
        limit: u32,
    ) -> Result<Vec<TopAccessPointRow>, AppError> {
        let db = &*self.db;
        let sql = r#"
            SELECT
                lm.access_point_id,
                ap.name AS access_point_name,
                ap.short_code AS short_code,
                COUNT(*)::BIGINT AS request_count,
                COALESCE(SUM(ltu.total_tokens), 0)::BIGINT AS total_tokens
            FROM log_metadata lm
            LEFT JOIN log_token_usage ltu ON ltu.log_id = lm.id
            LEFT JOIN access_points ap ON ap.id = lm.access_point_id
            WHERE lm.user_id = $1::uuid
              AND lm.access_point_id IS NOT NULL
              AND lm.timestamp >= $2::timestamptz
              AND lm.timestamp < $3::timestamptz
            GROUP BY lm.access_point_id, ap.name, ap.short_code
            ORDER BY total_tokens DESC
            LIMIT $4
        "#;

        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [
                user_id.into(),
                window.start.into(),
                window.end.into(),
                (limit as i64).into(),
            ],
        );

        let results = db.query_all_raw(stmt).await?;

        let rows = results
            .iter()
            .map(|row| {
                Ok(TopAccessPointRow {
                    access_point_id: row.try_get_by_index::<Uuid>(0)?,
                    name: row.try_get_by_index::<Option<String>>(1)?,
                    short_code: row.try_get_by_index::<Option<String>>(2)?,
                    request_count: row.try_get_by_index::<i64>(3)?,
                    total_tokens: row.try_get_by_index::<i64>(4)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(rows)
    }

    /// 用户视角调用质量指标。
    ///
    /// 1. 状态码分布通过 `SUM(CASE WHEN ...)` 在单次扫描中完成 4 类计数（2xx / 4xx / 5xx / 中断）。
    /// 2. `AVG(duration_ms)` 与 `PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms)`
    ///    在 0 行时返回 NULL，对应 Rust 侧 `Option<f64>`。
    /// 3. 仅查 `log_metadata`，无需联表 `log_token_usage`，索引 `idx_log_metadata_user_id` 适用。
    #[tracing::instrument(
        skip(self),
        fields(user_id = %user_id, window.start = %window.start, window.end = %window.end)
    )]
    async fn quality_metrics_for_user(
        &self,
        user_id: Uuid,
        window: &DashboardWindow,
    ) -> Result<QualityMetrics, AppError> {
        let db = &*self.db;
        let sql = r#"
            SELECT
                COUNT(*)::BIGINT AS total_count,
                COALESCE(SUM(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 ELSE 0 END), 0)::BIGINT AS success_count,
                COALESCE(SUM(CASE WHEN status_code >= 400 AND status_code < 500 THEN 1 ELSE 0 END), 0)::BIGINT AS client_error_count,
                COALESCE(SUM(CASE WHEN status_code >= 500 THEN 1 ELSE 0 END), 0)::BIGINT AS server_error_count,
                COALESCE(SUM(CASE WHEN is_interrupted THEN 1 ELSE 0 END), 0)::BIGINT AS interrupted_count,
                AVG(duration_ms)::FLOAT8 AS avg_duration_ms,
                PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms)::FLOAT8 AS p95_duration_ms
            FROM log_metadata
            WHERE user_id = $1::uuid
              AND timestamp >= $2::timestamptz
              AND timestamp < $3::timestamptz
        "#;

        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [user_id.into(), window.start.into(), window.end.into()],
        );

        let row = db
            .query_one_raw(stmt)
            .await?
            .ok_or_else(|| AppError::Internal("调用质量聚合查询无结果".to_string()))?;

        Ok(QualityMetrics {
            total_count: row.try_get_by_index::<i64>(0)?,
            success_count: row.try_get_by_index::<i64>(1)?,
            client_error_count: row.try_get_by_index::<i64>(2)?,
            server_error_count: row.try_get_by_index::<i64>(3)?,
            interrupted_count: row.try_get_by_index::<i64>(4)?,
            avg_duration_ms: row.try_get_by_index::<Option<f64>>(5)?,
            p95_duration_ms: row.try_get_by_index::<Option<f64>>(6)?,
        })
    }
}
