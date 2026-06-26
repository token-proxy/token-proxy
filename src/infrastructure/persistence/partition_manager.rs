use std::collections::{BTreeMap, HashSet};
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

/// 单条分区统计信息（来自 PostgreSQL 系统表查询）
#[derive(Debug, Clone)]
pub struct PartitionInfo {
    /// 分区名称（如 `log_metadata_2026_06`）
    pub partition_name: String,
    /// 所属父表（`log_metadata` 或 `log_contents`）
    pub parent_table: String,
    /// 磁盘占用（字节），来自 `pg_total_relation_size`，前端按 1024 进制格式化为 GiB
    pub size_bytes: i64,
    /// 估算行数，来自 `pg_class.reltuples`（近似值，非精确计数）
    pub row_count_estimate: i64,
}

/// 受分区管理的表名列表 — 仅 log_contents（log_requests 为普通表，无需分区）
const PARTITIONED_TABLES: &[&str] = &["log_contents"];

/// 分区管理器
///
/// 负责 `log_metadata` 和 `log_contents` 表的按月分区自动管理：
/// - 自动创建未来月份的分区
/// - 自动清理过期分区
/// - 通过 advisory lock 保证多副本部署安全
///
/// `log_token_usage` 不在此管理范围——词元用量数据需永久保留。
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
    pub async fn existing_partitions(&self, parent: &str) -> Result<Vec<String>, AppError> {
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
    /// 获取所有日志分区的磁盘占用和估算行数
    ///
    /// 通过 `pg_inherits` 和 `pg_class` 系统表查询 `log_metadata` 和 `log_contents` 的所有分区信息。
    /// `row_count_estimate` 来自 `pg_class.reltuples`，为 PostgreSQL 估算值，非精确计数。
    pub async fn get_partition_stats(&self) -> Result<Vec<PartitionInfo>, AppError> {
        let db = &*self.db;
        let sql = "SELECT inhrelid::regclass::text AS partition_name, \
                   inhparent::regclass::text AS parent_table, \
                   pg_total_relation_size(inhrelid) AS size_bytes, \
                   c.reltuples::bigint AS row_count_estimate \
                   FROM pg_inherits \
                   JOIN pg_class c ON c.oid = inhrelid \
                   WHERE inhparent = 'log_contents'::regclass \
                   ORDER BY parent_table, partition_name";
        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, sql, []);
        let rows = db
            .query_all_raw(stmt)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut partitions = Vec::new();
        for row in &rows {
            let partition: PartitionInfo = PartitionInfo {
                partition_name: row
                    .try_get_by_index(0)
                    .map_err(|e| AppError::Database(e.to_string()))?,
                parent_table: row
                    .try_get_by_index(1)
                    .map_err(|e| AppError::Database(e.to_string()))?,
                size_bytes: row
                    .try_get_by_index(2)
                    .map_err(|e| AppError::Database(e.to_string()))?,
                row_count_estimate: row
                    .try_get_by_index(3)
                    .map_err(|e| AppError::Database(e.to_string()))?,
            };
            partitions.push(partition);
        }
        Ok(partitions)
    }

    /// 删除指定父表的指定月份分区
    ///
    /// `year_month` 格式为 `YYYY-MM`，如 `2026-01`。
    /// 分区名格式为 `{parent_table}_{YYYY}_{MM}`。
    /// 返回被删除的分区名。
    pub async fn drop_partition(
        &self,
        parent_table: &str,
        year_month: &str,
    ) -> Result<String, AppError> {
        let db = &*self.db;
        // 将 YYYY-MM 转换为 YYYY_MM
        let ym = year_month.replace('-', "_");
        let partition_name = format!("{parent_table}_{ym}");
        let sql = format!("DROP TABLE IF EXISTS {partition_name}");
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            &sql,
            [],
        ))
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(partition_name)
    }

    /// 基于占用上限清理过期日志分区
    ///
    /// 获取所有日志分区，按月份聚合大小。若总占用超过 `cap_gb`，
    /// 从最早月份开始逐月删除分区（跳过当前月份），直到占用低于上限。
    /// 每月同时删除 `log_metadata` 和 `log_contents` 两个分区。
    ///
    /// 返回被删除的分区名列表。
    pub async fn run_storage_cap_cleanup(&self, cap_gb: u32) -> Result<Vec<String>, AppError> {
        let cap_bytes = (cap_gb as i64) * 1_073_741_824; // GB → bytes
        let stats = self.get_partition_stats().await?;

        // 按月份聚合所有分区的大小
        let mut monthly_sizes: BTreeMap<String, i64> = BTreeMap::new();
        for p in &stats {
            // 从分区名提取月份键
            if let Some(key) = extract_year_month(&p.partition_name) {
                *monthly_sizes.entry(key).or_insert(0) += p.size_bytes;
            }
        }

        // 计算总大小
        let total_size: i64 = monthly_sizes.values().sum();
        if total_size <= cap_bytes {
            return Ok(Vec::new());
        }

        // 从最老月份开始删除
        let now = Utc::now().naive_utc().date();
        let current_ym = format!("{:04}-{:02}", now.year(), now.month());

        let mut removed = Vec::new();
        let mut remaining = total_size;

        for (month, size) in &monthly_sizes {
            if remaining <= cap_bytes {
                break;
            }
            // 跳过当前月份
            if month == &current_ym {
                continue;
            }
            // 删除该月的 log_contents 分区（log_requests 不分片，无需清理）
            match self.drop_partition("log_contents", month).await {
                Ok(name) => removed.push(name),
                Err(e) => tracing::warn!(error = %e, month = %month, "删除 log_contents 分区失败"),
            }
            remaining -= size;
        }

        Ok(removed)
    }
}

/// 从分区名中提取 YYYY-MM 格式的月份标识
///
/// 分区名格式为 `{table}_{YYYY}_{MM}`，如 `log_metadata_2026_06`。
/// 无法解析时返回 None。
fn extract_year_month(partition_name: &str) -> Option<String> {
    // 从右侧找最后两个下划线分隔的部分
    let parts: Vec<&str> = partition_name.rsplitn(3, '_').collect();
    if parts.len() == 3 {
        let month_str = parts[0];
        let year_str = parts[1];
        if year_str.len() == 4 && month_str.len() == 2 {
            if let (Ok(y), Ok(m)) = (year_str.parse::<i32>(), month_str.parse::<u32>()) {
                if (1..=12).contains(&m) {
                    return Some(format!("{:04}-{:02}", y, m));
                }
            }
        }
    }
    None
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
