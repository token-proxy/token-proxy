//! 审计日志 Repository 实现（基础设施层）

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, PaginatorTrait, QueryOrder, Set,
    Statement,
};
use uuid::Uuid;

use crate::domain::log::audit_log::{ActiveModel, Column, Entity};
use crate::domain::log::{AuditLog, AuditLogQuery, AuditLogRepository, AuditLogWithUsername};
use crate::shared::error::AppError;
use crate::shared::types::PaginatedResult;

pub struct SeaOrmAuditLogRepository {
    db: Arc<DatabaseConnection>,
}

impl SeaOrmAuditLogRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        SeaOrmAuditLogRepository { db }
    }
}

#[async_trait]
impl AuditLogRepository for SeaOrmAuditLogRepository {
    async fn save(&self, log: &AuditLog) -> Result<(), AppError> {
        let active_model = ActiveModel {
            id: Set(log.id),
            operator_id: Set(log.operator_id),
            operator_type: Set(log.operator_type.clone()),
            action: Set(log.action.clone()),
            entity_type: Set(log.entity_type.clone()),
            entity_id: Set(log.entity_id),
            details: Set(log.details.clone()),
            timestamp: Set(log.timestamp),
        };

        Entity::insert(active_model).exec(&*self.db).await?;
        Ok(())
    }

    async fn find_all_paginated(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResult<AuditLog>, AppError> {
        let db = &*self.db;
        let paginator = Entity::find()
            .order_by_desc(Column::Timestamp)
            .paginate(db, page_size);

        let items = paginator.fetch_page(page.max(1) - 1).await?;
        let total = paginator.num_items().await?;

        Ok(PaginatedResult {
            items,
            total,
            page,
            page_size,
        })
    }

    async fn find_all_paginated_with_username(
        &self,
        page: u64,
        page_size: u64,
        query: &AuditLogQuery,
    ) -> Result<PaginatedResult<AuditLogWithUsername>, AppError> {
        let db = &*self.db;
        let page = page.max(1);
        let page_size = page_size.min(100);
        let offset = (page - 1) * page_size;
        let utc_offset = chrono::FixedOffset::east_opt(0).expect("UTC 偏移");

        let mut param_index = 1u32;

        // ─── COUNT 查询 SQL ───
        let mut count_sql =
            "SELECT COUNT(*)::BIGINT AS cnt FROM audit_logs al WHERE 1=1".to_string();
        let mut count_params: Vec<sea_orm::Value> = Vec::new();

        // ─── DATA 查询 SQL ───
        let mut data_sql = r#"
            SELECT
                al.id, al.operator_id, al.operator_type,
                al.action, al.entity_type, al.entity_id,
                al.details, al.timestamp,
                u.display_name AS username
            FROM audit_logs al
            LEFT JOIN users u ON al.operator_id = u.id
            WHERE 1=1
        "#
        .to_string();
        let mut data_params: Vec<sea_orm::Value> = Vec::new();

        // 1. 操作类型筛选（= ANY($N::varchar[])）
        if let Some(ref actions) = query.actions {
            if !actions.is_empty() {
                let p = format!("${}", param_index);
                param_index += 1;
                let cond = format!(" AND al.action = ANY({}::varchar[])", p);
                count_sql.push_str(&cond);
                data_sql.push_str(&cond);
                let action_strings: Vec<String> = actions.iter().map(|a| a.to_string()).collect();
                let val: sea_orm::Value = action_strings.into();
                count_params.push(val.clone());
                data_params.push(val);
            }
        }

        // 2. 实体类型筛选（= ANY($N::varchar[])）
        if let Some(ref entity_types) = query.entity_types {
            if !entity_types.is_empty() {
                let p = format!("${}", param_index);
                param_index += 1;
                let cond = format!(" AND al.entity_type = ANY({}::varchar[])", p);
                count_sql.push_str(&cond);
                data_sql.push_str(&cond);
                let type_strings: Vec<String> =
                    entity_types.iter().map(|t| t.to_string()).collect();
                let val: sea_orm::Value = type_strings.into();
                count_params.push(val.clone());
                data_params.push(val);
            }
        }

        // 3. 操作者 ID 精确匹配
        if let Some(ref operator_id) = query.operator_id {
            let p = format!("${}", param_index);
            param_index += 1;
            let cond = format!(" AND al.operator_id = {}", p);
            count_sql.push_str(&cond);
            data_sql.push_str(&cond);
            let val: sea_orm::Value = (*operator_id).into();
            count_params.push(val.clone());
            data_params.push(val);
        }

        // 4. 操作者类型筛选
        if let Some(ref operator_type) = query.operator_type {
            let p = format!("${}", param_index);
            param_index += 1;
            let cond = format!(" AND al.operator_type = {}", p);
            count_sql.push_str(&cond);
            data_sql.push_str(&cond);
            let val: sea_orm::Value = operator_type.clone().into();
            count_params.push(val.clone());
            data_params.push(val);
        }

        // 5. 时间范围起始
        if let Some(ref start_time) = query.start_time {
            let p = format!("${}", param_index);
            param_index += 1;
            let cond = format!(" AND al.timestamp >= {}::timestamptz", p);
            count_sql.push_str(&cond);
            data_sql.push_str(&cond);
            let val: sea_orm::Value = start_time.with_timezone(&utc_offset).into();
            count_params.push(val.clone());
            data_params.push(val);
        }

        // 6. 时间范围结束
        if let Some(ref end_time) = query.end_time {
            let p = format!("${}", param_index);
            param_index += 1;
            let cond = format!(" AND al.timestamp <= {}::timestamptz", p);
            count_sql.push_str(&cond);
            data_sql.push_str(&cond);
            let val: sea_orm::Value = end_time.with_timezone(&utc_offset).into();
            count_params.push(val.clone());
            data_params.push(val);
        }

        // ─── 排序 + 分页 ───
        let limit_placeholder = format!("${}", param_index);
        param_index += 1;
        let offset_placeholder = format!("${}", param_index);

        data_sql.push_str(&format!(
            " ORDER BY al.timestamp DESC LIMIT {} OFFSET {}",
            limit_placeholder, offset_placeholder
        ));
        data_params.push((page_size as i64).into());
        data_params.push((offset as i64).into());

        // ─── 执行 COUNT 查询 ───
        let count_stmt =
            Statement::from_sql_and_values(DbBackend::Postgres, &count_sql, count_params);
        let count_result = db
            .query_one_raw(count_stmt)
            .await?
            .ok_or_else(|| AppError::Internal("审计日志计数查询结果为空".to_string()))?;
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

        // ─── 执行 DATA 查询 ───
        let data_stmt = Statement::from_sql_and_values(DbBackend::Postgres, &data_sql, data_params);
        let results = db.query_all_raw(data_stmt).await?;

        let items: Vec<AuditLogWithUsername> = results
            .iter()
            .map(|row| {
                let timestamp_col: chrono::DateTime<chrono::FixedOffset> =
                    row.try_get_by_index(7)?;

                Ok(AuditLogWithUsername {
                    log: AuditLog {
                        id: row.try_get_by_index::<Uuid>(0)?,
                        operator_id: row.try_get_by_index::<Option<Uuid>>(1)?,
                        operator_type: row.try_get_by_index::<String>(2)?,
                        action: row.try_get_by_index::<String>(3)?,
                        entity_type: row.try_get_by_index::<String>(4)?,
                        entity_id: row.try_get_by_index::<Option<Uuid>>(5)?,
                        details: row.try_get_by_index::<Option<serde_json::Value>>(6)?,
                        timestamp: timestamp_col,
                    },
                    username: row.try_get_by_index::<Option<String>>(8)?,
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
}
