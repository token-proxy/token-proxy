use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Statement,
};
use uuid::Uuid;

use crate::domain::log::{LogContent, LogEntry, LogTokenUsage};
use crate::domain::log::{
    LogEntryWithTokenSummary, LogQuery, LogRepository, SessionQuery, SessionSummaryData,
};
use crate::domain::log::content::{
    ActiveModel as ContentActiveModel, Entity as ContentEntity,
};
use crate::domain::log::metadata::{ActiveModel, Column, Entity};
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
            .await?;

        match model {
            Some(m) => Ok(Some(m)),
            None => Ok(None),
        }
    }

    async fn find_by_session_id(&self, session_id: &str) -> Result<Vec<LogEntry>, AppError> {
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
            .await?;

        let models = paginator
            .fetch_page(page - 1)
            .await?;

        let items = models;

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
            .await?
            .is_some();

        let active_model: ActiveModel = entry.clone().into();

        if exists {
            let active_model = active_model.reset_all();
            Entity::update(active_model)
                .exec(db)
                .await?;
        } else {
            Entity::insert(active_model)
                .exec(db)
                .await?;
        }

        let result = Entity::find_by_id(entry.id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Internal("保存后无法查询到 LogEntry".to_string()))?;
        Ok(result)
    }

    async fn save_content(&self, content: &LogContent) -> Result<(), AppError> {
        let db = &*self.db;
        let active_model: ContentActiveModel = content.clone().into();
        ContentEntity::insert(active_model)
            .exec(db)
            .await?;
        Ok(())
    }

    async fn find_content_by_log_id(&self, log_id: Uuid) -> Result<Option<LogContent>, AppError> {
        let db = &*self.db;
        let model = ContentEntity::find_by_id(log_id)
            .one(db)
            .await?;

        match model {
            Some(m) => Ok(Some(m)),
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
            .await?;
        Ok(())
    }

    // ─── 新查询方法 ───

    async fn find_all_paginated_with_token_summary(
        &self,
        page: u64,
        page_size: u64,
        filter: &LogQuery,
    ) -> Result<PaginatedResult<LogEntryWithTokenSummary>, AppError> {
        let db = &*self.db;
        let page = page.max(1);
        let page_size = page_size.min(100);
        let offset = (page - 1) * page_size;
        let utc_offset = FixedOffset::east_opt(0).expect("UTC 偏移");

        // 构建 WHERE 条件参数
        let mut param_index = 1u32;

        // 先查询总数
        let mut count_sql = "SELECT COUNT(*)::BIGINT AS cnt FROM log_metadata lm WHERE 1=1".to_string();
        let mut count_params: Vec<sea_orm::Value> = Vec::new();

        // 数据查询 SQL
        let mut data_sql = r#"
            SELECT
                lm.id, lm.timestamp, lm.session_id, lm.user_id,
                lm.access_point_id, lm.provider_id, lm.account_id,
                lm.model_original, lm.model_mapped,
                lm.status_code, lm.duration_ms, lm.error_message,
                lm.request_index,
                lm.client_session_id, lm.client_app, lm.client_user_agent,
                lm.conversation_source,
                lm.agent_id,
                lm.has_error, lm.raw_content_available,
                lm.client_name, lm.client_version,
                lm.client_channel, lm.client_platform, lm.api_type,
                ltu.input_tokens, ltu.output_tokens, ltu.total_tokens
            FROM log_metadata lm
            LEFT JOIN log_token_usage ltu ON ltu.log_id = lm.id
            WHERE 1=1
        "#.to_string();
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
        let count_stmt = Statement::from_sql_and_values(DbBackend::Postgres, &count_sql, count_params);
        let count_result = db
            .query_one_raw(count_stmt)
            .await?
            .ok_or_else(|| AppError::Internal("计数查询结果为空".to_string()))?;
        let total: i64 = count_result
            .try_get_by_index(0)
            ?;
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
        let results = db
            .query_all_raw(data_stmt)
            .await?;

        let items: Vec<LogEntryWithTokenSummary> = results
            .iter()
            .map(|row| {
                let id: Uuid = row
                    .try_get_by_index::<Uuid>(0)
                    ?;

                let timestamp_col: chrono::DateTime<FixedOffset> = row
                    .try_get_by_index(1)
                    ?;

                Ok(LogEntryWithTokenSummary {
                    entry: LogEntry {
                        id,
                        timestamp: timestamp_col,
                        session_id: row
                            .try_get_by_index::<String>(2)
                            ?,
                        user_id: row
                            .try_get_by_index::<Option<Uuid>>(3)
                            ?,
                        access_point_id: row
                            .try_get_by_index::<Option<Uuid>>(4)
                            ?,
                        provider_id: row
                            .try_get_by_index::<Option<Uuid>>(5)
                            ?,
                        account_id: row
                            .try_get_by_index::<Option<Uuid>>(6)
                            ?,
                        model_original: row
                            .try_get_by_index::<Option<String>>(7)
                            ?,
                        model_mapped: row
                            .try_get_by_index::<Option<String>>(8)
                            ?,
                        status_code: row
                            .try_get_by_index::<Option<i16>>(9)
                            ?,
                        duration_ms: row
                            .try_get_by_index::<Option<i32>>(10)
                            ?,
                        error_message: row
                            .try_get_by_index::<Option<String>>(11)
                            ?,
                        request_index: row
                            .try_get_by_index::<i32>(12)
                            ?,
                        client_session_id: row
                            .try_get_by_index::<Option<String>>(13)
                            ?,
                        client_app: row
                            .try_get_by_index::<Option<String>>(14)
                            ?,
                        client_user_agent: row
                            .try_get_by_index::<Option<String>>(15)
                            ?,
                        conversation_source: row
                            .try_get_by_index::<String>(16)
                            ?,
                        agent_id: row
                            .try_get_by_index::<Option<String>>(17)
                            ?,
                        has_error: row
                            .try_get_by_index::<bool>(18)
                            ?,
                        raw_content_available: row
                            .try_get_by_index::<bool>(19)
                            ?,
                        client_name: row
                            .try_get_by_index::<Option<String>>(20)
                            ?,
                        client_version: row
                            .try_get_by_index::<Option<String>>(21)
                            ?,
                        client_channel: row
                            .try_get_by_index::<Option<String>>(22)
                            ?,
                        client_platform: row
                            .try_get_by_index::<Option<String>>(23)
                            ?,
                        api_type: row
                            .try_get_by_index::<String>(24)
                            ?,
                    },
                    input_tokens: row
                        .try_get_by_index::<Option<i32>>(25)
                        ?,
                    output_tokens: row
                        .try_get_by_index::<Option<i32>>(26)
                        ?,
                    total_tokens: row
                        .try_get_by_index::<Option<i32>>(27)
                        ?,
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
        let count_stmt = Statement::from_sql_and_values(DbBackend::Postgres, &count_sql, count_params);
        let count_result = db
            .query_one_raw(count_stmt)
            .await?
            .ok_or_else(|| AppError::Internal("会话计数查询结果为空".to_string()))?;
        let total: i64 = count_result
            .try_get_by_index(0)
            ?;
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
        let results = db
            .query_all_raw(data_stmt)
            .await?;

        let items: Vec<SessionSummaryData> = results
            .iter()
            .map(|row| {
                let start_time_col: chrono::DateTime<FixedOffset> = row
                    .try_get_by_index(3)
                    ?;

                Ok(SessionSummaryData {
                    session_id: row
                        .try_get_by_index::<String>(0)
                        ?,
                    user_id: row
                        .try_get_by_index::<Option<Uuid>>(1)
                        ?,
                    access_point_id: row
                        .try_get_by_index::<Option<Uuid>>(2)
                        ?,
                    start_time: start_time_col.to_utc(),
                    request_count: row
                        .try_get_by_index::<i64>(4)
                        ?,
                    total_input_tokens: row
                        .try_get_by_index::<i64>(5)
                        ?,
                    total_output_tokens: row
                        .try_get_by_index::<i64>(6)
                        ?,
                    total_cache_creation_input_tokens: row
                        .try_get_by_index::<i64>(7)
                        ?,
                    total_cache_read_input_tokens: row
                        .try_get_by_index::<i64>(8)
                        ?,
                    total_thinking_tokens: row
                        .try_get_by_index::<i64>(9)
                        ?,
                    total_tokens: row
                        .try_get_by_index::<i64>(10)
                        ?,
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
    ) -> Result<Option<(LogEntry, LogContent, Option<LogTokenUsage>)>, AppError> {
        let db = &*self.db;

        let sql = r#"
            SELECT
                lm.id, lm.timestamp, lm.session_id, lm.user_id,
                lm.access_point_id, lm.provider_id, lm.account_id,
                lm.model_original, lm.model_mapped,
                lm.status_code, lm.duration_ms, lm.error_message,
                lm.request_index,
                lm.client_session_id, lm.client_app, lm.client_user_agent,
                lm.conversation_source,
                lm.agent_id,
                lm.has_error, lm.raw_content_available,
                lm.client_name, lm.client_version,
                lm.client_channel, lm.client_platform, lm.api_type,
                lc.request_headers, lc.request_body, lc.response_body,
                ltu.id as usage_id, ltu.log_id as usage_log_id,
                ltu.input_tokens, ltu.output_tokens,
                ltu.cache_creation_input_tokens, ltu.cache_read_input_tokens,
                ltu.thinking_tokens, ltu.total_tokens, ltu.raw_usage,
                ltu.server_tool_usage, ltu.cache_creation,
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
        let result = db
            .query_one_raw(stmt)
            .await?;

        match result {
            Some(row) => {
                let timestamp_col: chrono::DateTime<FixedOffset> = row
                    .try_get_by_index(1)
                    ?;

                let entry = LogEntry {
                    id: row.try_get_by_index::<Uuid>(0)
                        ?,
                    timestamp: timestamp_col,
                    session_id: row.try_get_by_index::<String>(2)
                        ?,
                    user_id: row.try_get_by_index::<Option<Uuid>>(3)
                        ?,
                    access_point_id: row.try_get_by_index::<Option<Uuid>>(4)
                        ?,
                    provider_id: row.try_get_by_index::<Option<Uuid>>(5)
                        ?,
                    account_id: row.try_get_by_index::<Option<Uuid>>(6)
                        ?,
                    model_original: row.try_get_by_index::<Option<String>>(7)
                        ?,
                    model_mapped: row.try_get_by_index::<Option<String>>(8)
                        ?,
                    status_code: row.try_get_by_index::<Option<i16>>(9)
                        ?,
                    duration_ms: row.try_get_by_index::<Option<i32>>(10)
                        ?,
                    error_message: row.try_get_by_index::<Option<String>>(11)
                        ?,
                    request_index: row.try_get_by_index::<i32>(12)
                        ?,
                    client_session_id: row.try_get_by_index::<Option<String>>(13)
                        ?,
                    client_app: row.try_get_by_index::<Option<String>>(14)
                        ?,
                    client_user_agent: row.try_get_by_index::<Option<String>>(15)
                        ?,
                    conversation_source: row.try_get_by_index::<String>(16)
                        ?,
                    agent_id: row.try_get_by_index::<Option<String>>(17)
                        ?,
                    has_error: row.try_get_by_index::<bool>(18)
                        ?,
                    raw_content_available: row.try_get_by_index::<bool>(19)
                        ?,
                    client_name: row.try_get_by_index::<Option<String>>(20)
                        ?,
                    client_version: row.try_get_by_index::<Option<String>>(21)
                        ?,
                    client_channel: row.try_get_by_index::<Option<String>>(22)
                        ?,
                    client_platform: row.try_get_by_index::<Option<String>>(23)
                        ?,
                    api_type: row.try_get_by_index::<String>(24)
                        ?,
                };

                let content = LogContent {
                    log_id: entry.id,
                    request_headers: Some(row.try_get_by_index::<Option<serde_json::Value>>(25)
                        ?
                        .unwrap_or(serde_json::Value::Null)),
                    request_body: Some(row.try_get_by_index::<Option<serde_json::Value>>(26)
                        ?
                        .unwrap_or(serde_json::Value::Null)),
                    response_body: Some(row.try_get_by_index::<Option<String>>(27)
                        ?
                        .unwrap_or_default()),
                };

                // 检查是否有 token 用量（ltu.id 不为 NULL）
                let usage_id: Option<Uuid> = row.try_get_by_index::<Option<Uuid>>(28)
                    ?;

                let usage = if let Some(uid) = usage_id {
                    let usage_ts_col: chrono::DateTime<FixedOffset> = row
                        .try_get_by_index(40)
                        ?;
                    let usage_created_col: chrono::DateTime<FixedOffset> = row
                        .try_get_by_index(50)
                        ?;

                    Some(LogTokenUsage {
                        id: uid,
                        log_id: row.try_get_by_index::<Option<Uuid>>(29)
                            ?
                            .unwrap_or(entry.id),
                        input_tokens: row.try_get_by_index::<i32>(30)
                            ?,
                        output_tokens: row.try_get_by_index::<i32>(31)
                            ?,
                        cache_creation_input_tokens: row.try_get_by_index::<i32>(32)
                            ?,
                        cache_read_input_tokens: row.try_get_by_index::<i32>(33)
                            ?,
                        thinking_tokens: row.try_get_by_index::<i32>(34)
                            ?,
                        total_tokens: row.try_get_by_index::<i32>(35)
                            ?,
                        raw_usage: row.try_get_by_index::<Option<serde_json::Value>>(36)
                            ?,
                        server_tool_usage: row.try_get_by_index::<Option<serde_json::Value>>(37)
                            ?,
                        cache_creation: row.try_get_by_index::<Option<serde_json::Value>>(38)
                            ?,
                        session_id: row.try_get_by_index::<Option<String>>(39)
                            ?
                            .unwrap_or_default(),
                        timestamp: usage_ts_col,
                        user_id: row.try_get_by_index::<Option<Uuid>>(41)
                            ?,
                        access_point_id: row.try_get_by_index::<Option<Uuid>>(42)
                            ?,
                        provider_id: row.try_get_by_index::<Option<Uuid>>(43)
                            ?,
                        account_id: row.try_get_by_index::<Option<Uuid>>(44)
                            ?,
                        model_original: row.try_get_by_index::<Option<String>>(45)
                            ?,
                        model_mapped: row.try_get_by_index::<Option<String>>(46)
                            ?,
                        conversation_source: row.try_get_by_index::<Option<String>>(47)
                            ?,
                        agent_id: row.try_get_by_index::<Option<String>>(48)
                            ?,
                        agent_type: row.try_get_by_index::<Option<String>>(49)
                            ?,
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

    // ─── 统计方法 ───

    async fn count_total(&self) -> Result<u64, AppError> {
        let db = &*self.db;
        let count = Entity::find()
            .count(db)
            .await?;
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
            .query_all_raw(stmt)
            .await?;

        let mut data = Vec::new();
        for row in &results {
            let day_str: String = row
                .try_get_by_index(0)
                ?;
            let day = NaiveDate::parse_from_str(&day_str, "%Y-%m-%d")
                .map_err(|e| AppError::Internal(format!("日期解析失败: {}", e)))?;
            let count: i64 = row
                .try_get_by_index(1)
                ?;
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

        let stmt =
            Statement::from_sql_and_values(DbBackend::Postgres, sql, [(limit as i64).into()]);

        let results = db
            .query_all_raw(stmt)
            .await?;

        let mut data = Vec::new();
        for row in &results {
            let id: Uuid = row
                .try_get_by_index(0)
                ?;
            let count: i64 = row
                .try_get_by_index(1)
                ?;
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

        let stmt =
            Statement::from_sql_and_values(DbBackend::Postgres, sql, [(limit as i64).into()]);

        let results = db
            .query_all_raw(stmt)
            .await?;

        let mut data = Vec::new();
        for row in &results {
            let model: String = row
                .try_get_by_index(0)
                ?;
            let count: i64 = row
                .try_get_by_index(1)
                ?;
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
            .query_all_raw(stmt)
            .await?;

        let count: i64 = results
            .first()
            .ok_or_else(|| AppError::Internal("查询结果为空".to_string()))?
            .try_get_by_index(0)
            ?;

        Ok(count as u64)
    }
}
