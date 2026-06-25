/**
 * 审计日志相关 TypeScript 类型定义。
 *
 * 与后端 `src/domain/log/` 的 Rust 结构体一一对应，
 * 字段命名遵循后端 serde 序列化结果（snake_case）。
 */

/** 审计日志列表项（镜像后端 AuditLogWithUsername + AuditLog） */
export interface AuditLogItem {
  id: string;
  timestamp: string;
  operator_id: string | null;
  /** 操作者类型（"user" | "system"） */
  operator_type: string;
  /** 操作者用户名（来自 users 表 LEFT JOIN），为 null 时表示系统操作 */
  operator_name: string | null;
  /** 操作类型（snake_case 原始值，如 "create"、"update"） */
  action: string;
  /** 实体类型（snake_case 原始值，如 "access_point"、"user"） */
  entity_type: string;
  entity_id: string | null;
  /** 操作详情 JSON，可能为 null */
  details: Record<string, unknown> | null;
}

/** 审计日志筛选条件 */
export interface AuditLogFilters {
  startTime?: string;
  endTime?: string;
  actions?: string[];
  entityTypes?: string[];
  operatorId?: string;
  /** 操作者类型筛选（"user" 或 "system"） */
  operatorType?: string;
}
