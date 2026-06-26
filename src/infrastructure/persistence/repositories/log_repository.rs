//! 日志 Repository 实现（基础设施层）
//!
//! 实现了 `LogRepository` trait 的所有方法，基于 `log_requests` 单表。
//! LogRequest 自含所有标量字段（包括词元用量），所有查询无需 LEFT JOIN。
//! 内容详情按需 LEFT JOIN `log_contents`。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, Statement,
};
use uuid::Uuid;

use crate::domain::log::content::{ActiveModel as ContentActiveModel, Entity as ContentEntity};
use crate::domain::log::request::{ActiveModel, Column, Entity};
use crate::domain::log::{
    DashboardWindow, HeatmapCell, KpiAggregate, LogQuery, LogRepository, ModelTokenUsage,
    QualityMetrics, SessionQuery, SessionSummaryData, SparklineBucket, TopAccessPointRow,
    TopModelRow, UsageTrendBucket,
};
use crate::domain::log::{LogContent, LogRequest};
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
    async fn find_by_id(&self, id: Uuid) -> Result<Option<LogRequest>, AppError> {
        let db = &*self.db;
        let model = Entity::find_by_id(id).one(db).await?;
        match model {
            Some(m) => Ok(Some(m)),
            None => Ok(None),
        }
    }

    async fn find_by_session_id(&self, session_id: &str) -> Result<Vec<LogRequest>, AppError> {
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
    ) -> Result<PaginatedResult<LogRequest>, AppError> {
        let db = &*self.db;
        let page = page.max(1);
        let page_size = page_size.min(100);

        let mut params: Vec<sea_orm::Value> = Vec::new();
        let mut param_idx = 1u32;

        // 构建 WHERE 条件
        let mut where_clauses = Vec::new();

        if let Some(ref session_id) = filter.session_id {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.session_id = {}", p));
            params.push(session_id.clone().into());
        }
        if let Some(user_id) = &filter.user_id {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.user_id = {}", p));
            params.push((*user_id).into());
        }
        if let Some(access_point_id) = &filter.access_point_id {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.access_point_id = {}", p));
            params.push((*access_point_id).into());
        }
        if let Some(start_time) = &filter.start_time {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.timestamp >= {}::timestamptz", p));
            params.push(start_time.to_rfc3339().into());
        }
        if let Some(end_time) = &filter.end_time {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.timestamp <= {}::timestamptz", p));
            params.push(end_time.to_rfc3339().into());
        }
        if let Some(status_code) = &filter.status_code {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.status_code = {}", p));
            params.push((*status_code).into());
        }
        if let Some(provider_id) = &filter.provider_id {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.provider_id = {}", p));
            params.push((*provider_id).into());
        }
        if let Some(account_id) = &filter.account_id {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.account_id = {}", p));
            params.push((*account_id).into());
        }
        if let Some(is_interrupted) = &filter.is_interrupted {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.is_interrupted = {}", p));
            params.push((*is_interrupted).into());
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        // 总数查询
        let count_sql = format!(
            "SELECT COUNT(*)::BIGINT AS cnt FROM log_requests lr {}",
            where_sql
        );
        let count_stmt =
            Statement::from_sql_and_values(DbBackend::Postgres, &count_sql, params.clone());
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

        // 数据查询（单表，无需 JOIN）
        let offset = (page - 1) * page_size;
        let limit_p = format!("${}", param_idx);
        param_idx += 1;
        let offset_p = format!("${}", param_idx);
        params.push((page_size as i64).into());
        params.push((offset as i64).into());

        let data_sql = format!(
            r#"
            SELECT
                lr.id, lr.timestamp, lr.session_id, lr.user_id,
                lr.access_point_id, lr.provider_id, lr.account_id,
                lr.model_original, lr.model_mapped, lr.model_normalized,
                lr.status_code, lr.duration_ms, lr.error_message,
                lr.client_user_agent,
                lr.conversation_source,
                lr.agent_id,
                lr.has_error, lr.is_interrupted,
                lr.client_version,
                lr.api_type, lr.client_type,
                lr.input_tokens, lr.output_tokens,
                lr.cache_creation_input_tokens, lr.cache_read_input_tokens,
                lr.thinking_tokens, lr.total_tokens,
                lr.raw_usage, lr.server_tool_usage, lr.cache_creation,
                lr.agent_type, lr.created_at
            FROM log_requests lr
            {}
            ORDER BY lr.timestamp DESC
            LIMIT {} OFFSET {}
            "#,
            where_sql, limit_p, offset_p
        );

        let data_stmt = Statement::from_sql_and_values(DbBackend::Postgres, &data_sql, params);
        let results = db.query_all_raw(data_stmt).await?;

        let items: Vec<LogRequest> = results
            .iter()
            .map(|row| {
                let timestamp_col: DateTime<FixedOffset> = row.try_get_by_index(1)?;
                let created_at_col: DateTime<FixedOffset> = row.try_get_by_index(31)?;

                Ok(LogRequest {
                    id: row.try_get_by_index::<Uuid>(0)?,
                    timestamp: timestamp_col,
                    session_id: row.try_get_by_index::<String>(2)?,
                    user_id: row.try_get_by_index::<Option<Uuid>>(3)?,
                    access_point_id: row.try_get_by_index::<Option<Uuid>>(4)?,
                    provider_id: row.try_get_by_index::<Option<Uuid>>(5)?,
                    account_id: row.try_get_by_index::<Option<Uuid>>(6)?,
                    model_original: row.try_get_by_index::<Option<String>>(7)?,
                    model_mapped: row.try_get_by_index::<Option<String>>(8)?,
                    model_normalized: row.try_get_by_index::<String>(9)?,
                    status_code: row.try_get_by_index::<Option<i16>>(10)?,
                    duration_ms: row.try_get_by_index::<Option<i32>>(11)?,
                    error_message: row.try_get_by_index::<Option<String>>(12)?,
                    client_user_agent: row.try_get_by_index::<Option<String>>(13)?,
                    conversation_source: row.try_get_by_index::<String>(14)?,
                    agent_id: row.try_get_by_index::<Option<String>>(15)?,
                    has_error: row.try_get_by_index::<bool>(16)?,
                    is_interrupted: row.try_get_by_index::<bool>(17)?,
                    client_version: row.try_get_by_index::<Option<String>>(18)?,
                    api_type: row.try_get_by_index::<String>(19)?,
                    client_type: row.try_get_by_index::<String>(20)?,
                    input_tokens: row.try_get_by_index::<i32>(21)?,
                    output_tokens: row.try_get_by_index::<i32>(22)?,
                    cache_creation_input_tokens: row.try_get_by_index::<i32>(23)?,
                    cache_read_input_tokens: row.try_get_by_index::<i32>(24)?,
                    thinking_tokens: row.try_get_by_index::<i32>(25)?,
                    total_tokens: row.try_get_by_index::<i32>(26)?,
                    raw_usage: row.try_get_by_index::<Option<serde_json::Value>>(27)?,
                    server_tool_usage: row.try_get_by_index::<Option<serde_json::Value>>(28)?,
                    cache_creation: row.try_get_by_index::<Option<serde_json::Value>>(29)?,
                    agent_type: row.try_get_by_index::<Option<String>>(30)?,
                    created_at: created_at_col,
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

    async fn save(&self, entry: &LogRequest) -> Result<LogRequest, AppError> {
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
            .ok_or_else(|| AppError::Internal("保存后无法查询到 LogRequest".to_string()))?;
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
        // 再删除 log_request
        Entity::delete_by_id(id).exec(db).await?;
        Ok(())
    }

    // ─── 会话查询 ───

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
        let mut param_idx = 1u32;
        let mut params: Vec<sea_orm::Value> = Vec::new();

        let mut where_clauses = Vec::new();

        if let Some(ref session_id) = filter.session_id {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.session_id = {}", p));
            params.push(session_id.clone().into());
        }
        if let Some(user_id) = &filter.user_id {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.user_id = {}", p));
            params.push((*user_id).into());
        }
        if let Some(access_point_id) = &filter.access_point_id {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.access_point_id = {}", p));
            params.push((*access_point_id).into());
        }
        if let Some(start_time) = &filter.start_time {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.timestamp >= {}::timestamptz", p));
            params.push(start_time.to_rfc3339().into());
        }
        if let Some(end_time) = &filter.end_time {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.timestamp <= {}::timestamptz", p));
            params.push(end_time.to_rfc3339().into());
        }
        if let Some(status_code) = &filter.status_code {
            let p = format!("${}", param_idx);
            param_idx += 1;
            where_clauses.push(format!("lr.status_code = {}", p));
            params.push((*status_code).into());
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        // 总数查询
        let count_sql = format!(
            "SELECT COUNT(*)::BIGINT FROM (SELECT 1 FROM log_requests lr {} GROUP BY lr.session_id) sub",
            where_sql
        );
        let count_stmt =
            Statement::from_sql_and_values(DbBackend::Postgres, &count_sql, params.clone());
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

        // 数据查询（单表聚合）
        let limit_p = format!("${}", param_idx);
        param_idx += 1;
        let offset_p = format!("${}", param_idx);
        params.push((page_size as i64).into());
        params.push((offset as i64).into());

        let data_sql = format!(
            r#"
            SELECT
                lr.session_id,
                MIN(lr.user_id::text)::uuid as user_id,
                MIN(lr.access_point_id::text)::uuid as access_point_id,
                MIN(lr.timestamp) as start_time,
                CAST(COUNT(*) AS BIGINT) as request_count,
                COALESCE(SUM(lr.input_tokens), 0)::BIGINT as total_input_tokens,
                COALESCE(SUM(lr.output_tokens), 0)::BIGINT as total_output_tokens,
                COALESCE(SUM(lr.cache_creation_input_tokens), 0)::BIGINT as total_cache_creation_input_tokens,
                COALESCE(SUM(lr.cache_read_input_tokens), 0)::BIGINT as total_cache_read_tokens,
                COALESCE(SUM(lr.thinking_tokens), 0)::BIGINT as total_thinking_tokens,
                COALESCE(SUM(lr.total_tokens), 0)::BIGINT as total_tokens
            FROM log_requests lr
            {}
            GROUP BY lr.session_id
            ORDER BY start_time DESC
            LIMIT {} OFFSET {}
            "#,
            where_sql, limit_p, offset_p
        );

        let data_stmt = Statement::from_sql_and_values(DbBackend::Postgres, &data_sql, params);
        let results = db.query_all_raw(data_stmt).await?;

        let items: Vec<SessionSummaryData> = results
            .iter()
            .map(|row| {
                let start_time_col: DateTime<FixedOffset> = row.try_get_by_index(3)?;
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
    ) -> Result<Option<(LogRequest, LogContent)>, AppError> {
        let db = &*self.db;

        let sql = r#"
            SELECT
                lr.id, lr.timestamp, lr.session_id, lr.user_id,
                lr.access_point_id, lr.provider_id, lr.account_id,
                lr.model_original, lr.model_mapped, lr.model_normalized,
                lr.status_code, lr.duration_ms, lr.error_message,
                lr.client_user_agent,
                lr.conversation_source,
                lr.agent_id,
                lr.has_error, lr.is_interrupted,
                lr.client_version,
                lr.api_type, lr.client_type,
                lr.input_tokens, lr.output_tokens,
                lr.cache_creation_input_tokens, lr.cache_read_input_tokens,
                lr.thinking_tokens, lr.total_tokens,
                lr.raw_usage, lr.server_tool_usage, lr.cache_creation,
                lr.agent_type, lr.created_at,
                lc.request_headers, lc.request_body, lc.response_body, lc.response_headers
            FROM log_requests lr
            LEFT JOIN log_contents lc ON lc.log_id = lr.id
            WHERE lr.id = $1::uuid
        "#;

        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, sql, [id.into()]);
        let result = db.query_one_raw(stmt).await?;

        match result {
            Some(row) => {
                let timestamp_col: DateTime<FixedOffset> = row.try_get_by_index(1)?;
                let created_at_col: DateTime<FixedOffset> = row.try_get_by_index(31)?;

                let entry = LogRequest {
                    id: row.try_get_by_index::<Uuid>(0)?,
                    timestamp: timestamp_col,
                    session_id: row.try_get_by_index::<String>(2)?,
                    user_id: row.try_get_by_index::<Option<Uuid>>(3)?,
                    access_point_id: row.try_get_by_index::<Option<Uuid>>(4)?,
                    provider_id: row.try_get_by_index::<Option<Uuid>>(5)?,
                    account_id: row.try_get_by_index::<Option<Uuid>>(6)?,
                    model_original: row.try_get_by_index::<Option<String>>(7)?,
                    model_mapped: row.try_get_by_index::<Option<String>>(8)?,
                    model_normalized: row.try_get_by_index::<String>(9)?,
                    status_code: row.try_get_by_index::<Option<i16>>(10)?,
                    duration_ms: row.try_get_by_index::<Option<i32>>(11)?,
                    error_message: row.try_get_by_index::<Option<String>>(12)?,
                    client_user_agent: row.try_get_by_index::<Option<String>>(13)?,
                    conversation_source: row.try_get_by_index::<String>(14)?,
                    agent_id: row.try_get_by_index::<Option<String>>(15)?,
                    has_error: row.try_get_by_index::<bool>(16)?,
                    is_interrupted: row.try_get_by_index::<bool>(17)?,
                    client_version: row.try_get_by_index::<Option<String>>(18)?,
                    api_type: row.try_get_by_index::<String>(19)?,
                    client_type: row.try_get_by_index::<String>(20)?,
                    input_tokens: row.try_get_by_index::<i32>(21)?,
                    output_tokens: row.try_get_by_index::<i32>(22)?,
                    cache_creation_input_tokens: row.try_get_by_index::<i32>(23)?,
                    cache_read_input_tokens: row.try_get_by_index::<i32>(24)?,
                    thinking_tokens: row.try_get_by_index::<i32>(25)?,
                    total_tokens: row.try_get_by_index::<i32>(26)?,
                    raw_usage: row.try_get_by_index::<Option<serde_json::Value>>(27)?,
                    server_tool_usage: row.try_get_by_index::<Option<serde_json::Value>>(28)?,
                    cache_creation: row.try_get_by_index::<Option<serde_json::Value>>(29)?,
                    agent_type: row.try_get_by_index::<Option<String>>(30)?,
                    created_at: created_at_col,
                };

                let content = LogContent {
                    log_id: entry.id,
                    timestamp: entry.timestamp,
                    request_headers: row.try_get_by_index::<Option<serde_json::Value>>(32)?,
                    request_body: row.try_get_by_index::<Option<serde_json::Value>>(33)?,
                    response_body: row.try_get_by_index::<Option<String>>(34)?,
                    response_headers: row.try_get_by_index::<Option<serde_json::Value>>(35)?,
                };

                Ok(Some((entry, content)))
            }
            None => Ok(None),
        }
    }

    // ─── Dashboard 聚合查询（个人视角，所有方法均按 user_id 过滤）───

    /// KPI 聚合：单次 SQL 返回个人视角的请求数与 6 类词元 SUM。
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
                COUNT(DISTINCT lr.session_id)::BIGINT AS session_count,
                COALESCE(SUM(lr.total_tokens), 0)::BIGINT AS total_tokens,
                COALESCE(SUM(lr.input_tokens), 0)::BIGINT AS input_tokens,
                COALESCE(SUM(lr.output_tokens), 0)::BIGINT AS output_tokens,
                COALESCE(SUM(lr.cache_creation_input_tokens), 0)::BIGINT AS cache_creation_tokens,
                COALESCE(SUM(lr.cache_read_input_tokens), 0)::BIGINT AS cache_read_tokens,
                COALESCE(SUM(lr.thinking_tokens), 0)::BIGINT AS thinking_tokens,
                COALESCE(SUM(lr.input_tokens + lr.cache_creation_input_tokens + lr.cache_read_input_tokens), 0)::BIGINT AS total_input_side_tokens
            FROM log_requests lr
            WHERE lr.user_id = $1::uuid
              AND lr.timestamp >= $2::timestamptz
              AND lr.timestamp < $3::timestamptz
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

    /// Sparkline 聚合：按 hour 或 day 分桶，用 generate_series 补齐空桶。
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
        tz: &str,
    ) -> Result<Vec<SparklineBucket>, AppError> {
        let db = &*self.db;

        let unit = if bucket_count == 24 { "hour" } else { "day" };

        let sql = format!(
            r#"
            WITH series AS (
                SELECT generate_series(
                    date_trunc('{unit}', $1::timestamptz AT TIME ZONE '{tz}'),
                    date_trunc('{unit}', $2::timestamptz AT TIME ZONE '{tz}' - interval '1 second'),
                    interval '1 {unit}'
                ) AT TIME ZONE 'UTC' AS bucket_start
            ), data AS (
                SELECT
                    date_trunc('{unit}', lr.timestamp AT TIME ZONE '{tz}') AT TIME ZONE 'UTC' AS bucket_start,
                    COUNT(*)::BIGINT AS request_count,
                    COALESCE(SUM(lr.total_tokens), 0)::BIGINT AS total_tokens
                FROM log_requests lr
                WHERE lr.timestamp >= $1::timestamptz
                  AND lr.timestamp < $2::timestamptz
                  AND lr.user_id = $3::uuid
                GROUP BY 1
            )
            SELECT
                s.bucket_start,
                COALESCE(d.request_count, 0)::BIGINT,
                COALESCE(d.total_tokens, 0)::BIGINT
            FROM series s
            LEFT JOIN data d USING (bucket_start)
            ORDER BY s.bucket_start
            "#,
            tz = tz
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

    /// 用量趋势聚合：按日补齐请求数与词元分项，单表聚合。
    #[tracing::instrument(
        skip(self),
        fields(user_id = %user_id, window.start = %window.start, window.end = %window.end)
    )]
    async fn usage_trends_for_user(
        &self,
        user_id: Uuid,
        window: &DashboardWindow,
        tz: &str,
    ) -> Result<Vec<UsageTrendBucket>, AppError> {
        let db = &*self.db;

        // ─── 主查询：按日聚合请求数与词元分项（单表）───
        let sql = format!(
            r#"
            WITH series AS (
                SELECT generate_series(
                    date_trunc('day', $2::timestamptz AT TIME ZONE '{tz}'),
                    date_trunc('day', $3::timestamptz AT TIME ZONE '{tz}' - interval '1 second'),
                    interval '1 day'
                ) AT TIME ZONE 'UTC' AS bucket_start
            ), data AS (
                SELECT
                    date_trunc('day', lr.timestamp AT TIME ZONE '{tz}') AT TIME ZONE 'UTC' AS bucket_start,
                    COUNT(*)::BIGINT AS request_count,
                    COUNT(DISTINCT lr.session_id)::BIGINT AS session_count,
                    COALESCE(SUM(lr.total_tokens), 0)::BIGINT AS total_tokens,
                    COALESCE(SUM(lr.input_tokens), 0)::BIGINT AS input_tokens,
                    COALESCE(SUM(lr.output_tokens), 0)::BIGINT AS output_tokens,
                    COALESCE(SUM(lr.cache_creation_input_tokens), 0)::BIGINT AS cache_creation_tokens,
                    COALESCE(SUM(lr.cache_read_input_tokens), 0)::BIGINT AS cache_read_tokens,
                    COALESCE(SUM(lr.thinking_tokens), 0)::BIGINT AS thinking_tokens
                FROM log_requests lr
                WHERE lr.user_id = $1::uuid
                  AND lr.timestamp >= $2::timestamptz
                  AND lr.timestamp < $3::timestamptz
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
            "#,
            tz = tz
        );

        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [user_id.into(), window.start.into(), window.end.into()],
        );

        let results = db.query_all_raw(stmt).await?;
        let mut buckets: Vec<UsageTrendBucket> = results
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
                    per_model: Vec::new(),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        // ─── 模型维度查询：按日 + model_normalized 聚合总词元 ───
        let model_sql = format!(
            r#"
            SELECT
                date_trunc('day', lr.timestamp AT TIME ZONE '{tz}') AT TIME ZONE 'UTC' AS bucket_start,
                lr.model_normalized AS model,
                COALESCE(SUM(lr.total_tokens), 0)::BIGINT AS total_tokens
            FROM log_requests lr
            WHERE lr.user_id = $1::uuid
              AND lr.timestamp >= $2::timestamptz
              AND lr.timestamp < $3::timestamptz
            GROUP BY 1, 2
            ORDER BY 1, 3 DESC
            "#,
            tz = tz
        );

        let model_stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            model_sql,
            [user_id.into(), window.start.into(), window.end.into()],
        );

        let model_results = db.query_all_raw(model_stmt).await?;

        let mut model_map: HashMap<DateTime<Utc>, Vec<ModelTokenUsage>> = HashMap::new();
        for row in &model_results {
            let bucket_start: DateTime<FixedOffset> = row.try_get_by_index(0)?;
            let model: String = row.try_get_by_index(1)?;
            let total_tokens: i64 = row.try_get_by_index(2)?;
            model_map
                .entry(bucket_start.with_timezone(&Utc))
                .or_default()
                .push(ModelTokenUsage {
                    model,
                    total_tokens,
                });
        }

        for bucket in &mut buckets {
            if let Some(models) = model_map.remove(&bucket.bucket_start) {
                bucket.per_model = models;
            }
        }

        Ok(buckets)
    }

    /// 用户日级 365 天词元热力图。
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
                    date_trunc('day', lr.timestamp AT TIME ZONE '{tz}') AS day_local,
                    COUNT(*)::BIGINT AS request_count,
                    COALESCE(SUM(lr.total_tokens), 0)::BIGINT AS total_tokens
                FROM log_requests lr
                WHERE lr.user_id = $2::uuid
                  AND lr.timestamp >= ($1::timestamptz - interval '365 days')
                  AND lr.timestamp <= $1::timestamptz
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

    /// 用户视角模型排行 Top N（按 model_normalized 分组）。
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
                lr.model_normalized AS model,
                COUNT(*)::BIGINT AS request_count,
                COALESCE(SUM(lr.total_tokens), 0)::BIGINT AS total_tokens
            FROM log_requests lr
            WHERE lr.user_id = $1::uuid
              AND lr.timestamp >= $2::timestamptz
              AND lr.timestamp < $3::timestamptz
            GROUP BY lr.model_normalized
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
                lr.access_point_id,
                ap.name AS access_point_name,
                ap.short_code AS short_code,
                COUNT(*)::BIGINT AS request_count,
                COALESCE(SUM(lr.total_tokens), 0)::BIGINT AS total_tokens
            FROM log_requests lr
            LEFT JOIN access_points ap ON ap.id = lr.access_point_id
            WHERE lr.user_id = $1::uuid
              AND lr.access_point_id IS NOT NULL
              AND lr.timestamp >= $2::timestamptz
              AND lr.timestamp < $3::timestamptz
            GROUP BY lr.access_point_id, ap.name, ap.short_code
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
            FROM log_requests
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
