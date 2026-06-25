//! 日志统计相关 DTO — application/system/dto/
//!
//! 所有磁盘占用字段均为原始字节数（来自 `pg_total_relation_size`），
//! 前端负责按 1024 进制格式化为 GiB / MiB / KiB。

use serde::Serialize;

/// 单条分区信息
#[derive(Debug, Clone, Serialize)]
pub struct PartitionInfo {
    /// 分区名称（如 `log_metadata_2026_06`）
    pub partition_name: String,
    /// 所属父表（`log_metadata` 或 `log_contents`）
    pub parent_table: String,
    /// 磁盘占用（字节），来自 `pg_total_relation_size`
    pub size_bytes: i64,
    /// 估算行数，来自 `pg_class.reltuples`（近似值，非精确计数）
    pub row_count_estimate: i64,
}

/// 月度分区汇总信息
///
/// 按月份合并 `log_metadata` 和 `log_contents` 两个分区的磁盘占用，
/// 聚合为单月总计。
#[derive(Debug, Clone, Serialize)]
pub struct MonthlySummary {
    /// 月份标识（如 `2026-06`）
    pub month: String,
    /// 该月所有分区磁盘占用总和（字节）
    pub size_bytes: i64,
    /// 该月所有分区的估算行数总和（来自 pg_class.reltuples，近似值）
    pub row_count_estimate: i64,
}

/// 日志统计完整响应
#[derive(Debug, Clone, Serialize)]
pub struct LogStatsResponse {
    /// 分区列表
    pub partitions: Vec<PartitionInfo>,
    /// 按月汇总（按 month 升序排列）
    pub monthly_summary: Vec<MonthlySummary>,
    /// 分区总数
    pub total_partitions: usize,
    /// 日志总磁盘占用（字节）
    pub total_size_bytes: i64,
}
