use std::collections::HashSet;
use std::sync::Arc;

use chrono::{Datelike, Utc};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};

use crate::shared::error::AppError;

/// 分区维护结果
#[derive(Debug, Clone)]
pub struct PartitionResult {
    /// 本次创建的分区名列表
    pub created: Vec<String>,
    /// 本次删除的分区名列表
    pub dropped: Vec<String>,
}

/// 受分区管理的表名列表
const PARTITIONED_TABLES: &[&str] = &["log_metadata", "log_contents"];

/// 分区管理器
///
/// 负责 `log_metadata` 和 `log_contents` 表的按月分区自动管理：
/// - 自动创建未来月份的分区
/// - 自动清理过期分区
/// - 通过 advisory lock 保证多副本部署安全
///
/// `log_token_usage` 不在此管理范围——token 用量数据需永久保留。
pub struct PartitionManager {
    db: Arc<DatabaseConnection>,
    premake_months: u32,
}

/// 从 (year, month) 向前（n > 0）或向后（n < 0）调整 n 个月
fn add_months(year: i32, month: u32, n: i32) -> (i32, u32) {
    let total_months = (year * 12 + month as i32 - 1) + n;
    let y = total_months.div_euclid(12);
    let m = (total_months.rem_euclid(12) + 1) as u32;
    (y, m)
}

impl PartitionManager {
    pub fn new(db: Arc<DatabaseConnection>, premake_months: u32) -> Self {
        PartitionManager { db, premake_months }
    }

    /// 查询指定父表的现有分区名
    ///
    /// 通过 `pg_inherits` 系统表查询指定表的所有直接继承分区。
    /// `parent` 参数为表名（如 `log_metadata`）。
    async fn existing_partitions(&self, parent: &str) -> Result<Vec<String>, AppError> {
        let db = &*self.db;
        let sql = format!(
            "SELECT inhrelid::regclass::text AS partition_name \
             FROM pg_inherits \
             WHERE inhparent = '{parent}'::regclass"
        );
        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, &sql, []);
        let rows = db
            .query_all_raw(stmt)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut partitions = Vec::new();
        for row in &rows {
            let name: String = row
                .try_get_by_index(0)
                .map_err(|e| AppError::Database(e.to_string()))?;
            partitions.push(name);
        }
        Ok(partitions)
    }

    /// 执行分区维护
    ///
    /// 对每张受管表：
    /// 1. 查询现有分区
    /// 2. 确定需要覆盖的月份范围：从（当前月 - retention_months + 1）到（当前月 + premake_months）
    /// 3. 对缺失的未来月份执行 CREATE TABLE ... PARTITION OF
    /// 4. 对过期的分区执行 DROP TABLE
    pub async fn run_maintenance(
        &self,
        retention_months: u32,
    ) -> Result<PartitionResult, AppError> {
        let db = &*self.db;
        let now = Utc::now().naive_utc().date();
        let current_year = now.year();
        let current_month = now.month();

        // 计算需要覆盖的月份范围
        let retention_offset = retention_months as i32 - 1;
        let (start_year, start_month) =
            add_months(current_year, current_month, -(retention_offset));
        let (end_year, end_month) =
            add_months(current_year, current_month, self.premake_months as i32);

        let mut created = Vec::new();
        let mut dropped = Vec::new();

        for table_name in PARTITIONED_TABLES {
            let existing = self.existing_partitions(table_name).await?;
            let existing_set: HashSet<String> = existing.into_iter().collect();
            let prefix = format!("{table_name}_");

            // 删除过期分区（月份在起始范围之前的）
            for name in &existing_set {
                if let Some(ym) = name.strip_prefix(&prefix) {
                    if let Some((y_str, m_str)) = ym.split_once('_') {
                        if let (Ok(y), Ok(m)) = (y_str.parse::<i32>(), m_str.parse::<u32>()) {
                            if (y, m) < (start_year, start_month) {
                                let sql = format!("DROP TABLE IF EXISTS {name}");
                                db.execute_raw(Statement::from_sql_and_values(
                                    DbBackend::Postgres,
                                    &sql,
                                    [],
                                ))
                                .await
                                .map_err(|e| AppError::Database(e.to_string()))?;
                                dropped.push(name.clone());
                            }
                        }
                    }
                }
            }

            // 创建缺失的未来月份分区（从当前月到 end_month）
            let mut y = current_year;
            let mut m = current_month;
            loop {
                let name = format!("{prefix}{y:04}_{m:02}");
                if !existing_set.contains(&name) {
                    let date_str = format!("{y:04}-{m:02}-01");
                    let (ny, nm) = add_months(y, m, 1);
                    let next_date_str = format!("{ny:04}-{nm:02}-01");
                    let sql = format!(
                        "CREATE TABLE IF NOT EXISTS {name} PARTITION OF {table_name} \
                         FOR VALUES FROM ('{date_str}') TO ('{next_date_str}')"
                    );
                    db.execute_raw(Statement::from_sql_and_values(
                        DbBackend::Postgres,
                        &sql,
                        [],
                    ))
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;
                    created.push(name);
                }

                if y == end_year && m == end_month {
                    break;
                }

                let (ny, nm) = add_months(y, m, 1);
                y = ny;
                m = nm;
            }
        }

        Ok(PartitionResult { created, dropped })
    }

    /// 使用 advisory lock 执行分区维护
    ///
    /// 通过 PostgreSQL 的 pg_try_advisory_xact_lock 确保多副本部署场景下
    /// 只有一个实例执行分区维护，避免并发冲突。
    pub async fn run_maintenance_with_lock(
        &self,
        retention_months: u32,
    ) -> Result<PartitionResult, AppError> {
        let db = &*self.db;
        let sql = "SELECT CASE WHEN pg_try_advisory_xact_lock(123456789) THEN 1 ELSE 0 END";
        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, sql, []);
        let rows = db
            .query_all_raw(stmt)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let locked: i32 = rows
            .first()
            .ok_or_else(|| AppError::Internal("advisory lock 查询结果为空".to_string()))?
            .try_get_by_index(0)
            .map_err(|e| AppError::Database(e.to_string()))?;

        if locked == 1 {
            self.run_maintenance(retention_months).await
        } else {
            Ok(PartitionResult {
                created: Vec::new(),
                dropped: Vec::new(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_months_forward() {
        assert_eq!(add_months(2026, 6, 1), (2026, 7));
        assert_eq!(add_months(2026, 12, 1), (2027, 1));
        assert_eq!(add_months(2026, 1, 12), (2027, 1));
    }

    #[test]
    fn test_add_months_backward() {
        assert_eq!(add_months(2026, 6, -1), (2026, 5));
        assert_eq!(add_months(2026, 1, -1), (2025, 12));
        assert_eq!(add_months(2026, 6, -5), (2026, 1));
        assert_eq!(add_months(2026, 6, -6), (2025, 12));
    }

    #[test]
    fn test_add_months_zero() {
        assert_eq!(add_months(2026, 6, 0), (2026, 6));
        assert_eq!(add_months(2026, 1, 0), (2026, 1));
    }
}
