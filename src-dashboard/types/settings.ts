/** 单条分区信息 */
export interface PartitionInfo {
  /** 分区名称（如 `log_metadata_2026_06`） */
  partition_name: string;
  /** 所属父表（`log_metadata` 或 `log_contents`） */
  parent_table: string;
  /** 磁盘占用（字节），前端按 1024 进制格式化为 GiB */
  size_bytes: number;
  /** 估算行数（PostgreSQL reltuples 估算值） */
  row_count_estimate: number;
}

/** 月度分区汇总（按月份聚合 log_metadata + log_contents） */
export interface MonthlySummary {
  /** 月份标识（如 `2026-06`） */
  month: string;
  /** 该月所有分区磁盘占用总和（字节） */
  size_bytes: number;
  /** 该月所有分区的估算行数总和（近似值） */
  row_count_estimate: number;
}

/** GET /api/settings/log-stats 响应 */
export interface LogStatsResponse {
  /** 分区列表 */
  partitions: PartitionInfo[];
  /** 按月汇总（按 month 升序排列） */
  monthly_summary: MonthlySummary[];
  /** 分区总数 */
  total_partitions: number;
  /** 日志总磁盘占用（字节） */
  total_size_bytes: number;
}

/** 系统设置（GET /api/settings 响应） */
export interface Settings {
  /** 日志保留月数（1-36） */
  log_retention_months: number;
  /** 日志占用上限（GiB），null 表示不限制 */
  log_storage_cap_gb: number | null;
  /** 当前有日志数据的月份数 */
  log_month_count: number;
  /** 日志总磁盘占用（字节），前端按 1024 进制格式化为 GiB */
  total_size_bytes: number;
}

/** 更新系统设置请求体（PUT /api/settings） */
export interface UpdateSettingsRequest {
  /** 日志保留月数（1-36） */
  log_retention_months: number;
  /** 日志占用上限（GiB），null 表示不限制 */
  log_storage_cap_gb: number | null;
}
