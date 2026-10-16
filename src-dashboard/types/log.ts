/** 用户简要信息 */
export interface UserItem {
  id: string;
  username: string;
  display_name: string;
  status: string;
}

/** 接入点简要信息 */
export interface AccessPointItem {
  id: string;
  name: string;
  short_code: string;
}

/** 日志列表项摘要 */
export interface LogSummary {
  id: string;
  timestamp: string;
  session_id: string;
  user_id?: string | null;
  access_point_id?: string | null;
  /** 实际使用的服务商 ID */
  provider_id?: string | null;
  /** 实际使用的账号 ID */
  account_id?: string | null;
  /** 客户端是否中途中断连接 */
  is_interrupted: boolean;
  model_original?: string | null;
  model_mapped?: string | null;
  status_code?: number | null;
  duration_ms?: number | null;
  conversation_source: string;
  agent_id?: string | null;
  // 词元摘要
  token_input_tokens?: number | null;
  token_output_tokens?: number | null;
  token_cache_creation_input_tokens?: number | null;
  token_cache_read_input_tokens?: number | null;
  token_thinking_tokens?: number | null;
  token_total_tokens?: number | null;
  // 客户端信息
  client_type?: string | null;
  client_version?: string | null;
  client_user_agent?: string | null;
  // API 类型
  api_type?: string;
  /** 本次请求实际使用的协议（anthropic / openai / openai_response） */
  api_protocol?: string;
}

/** 日志原始详情（用于 /api/logs/{id}/raw） */
export interface LogDetail {
  id: string;
  timestamp: string;
  session_id: string;
  user_id?: string | null;
  access_point_id?: string | null;
  provider_id?: string | null;
  account_id?: string | null;
  model_original?: string | null;
  model_mapped?: string | null;
  status_code?: number | null;
  duration_ms?: number | null;
  error_message?: string | null;
  request_headers?: Record<string, unknown> | null;
  request_body?: Record<string, unknown> | null;
  response_body?: string | null;
}

/** 词元用量记录 */
export interface TokenUsage {
  id: string;
  log_id: string;
  session_id: string;
  timestamp: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_input_tokens: number;
  cache_read_input_tokens: number;
  thinking_tokens: number;
  total_tokens: number;
  raw_usage?: Record<string, unknown> | null;
  server_tool_usage?: Record<string, unknown> | null;
  cache_creation?: Record<string, unknown> | null;
}

/** 日志完整详情（用于独立详情页 /logs/:id） */
export interface LogDetailFull {
  id: string;
  timestamp: string;
  session_id: string;
  user_id?: string | null;
  user_name?: string | null;
  access_point_id?: string | null;
  access_point_name?: string | null;
  provider_id?: string | null;
  account_id?: string | null;
  model_original: string;
  model_mapped: string;
  status_code: number;
  duration_ms: number;
  error_message?: string | null;
  conversation_source: string;
  agent_id?: string | null;
  // 客户端信息
  client_type?: string | null;
  client_version?: string | null;
  client_user_agent?: string | null;
  // API 类型
  api_type?: string;
  /** 本次请求实际使用的协议（anthropic / openai / openai_response） */
  api_protocol?: string;
  /**
   * 原始内容是否落库（false = `log_contents` 无对应行，请求/响应原文已不可用）
   *
   * 旧后端不返回该字段，此时前端按请求/响应体是否为空兜底判断。
   */
  content_available?: boolean;
  // 请求 + 响应原始内容（前端自行解析）
  request_headers: Record<string, unknown> | null;
  response_headers: Record<string, unknown> | null;
  /** 请求体：正常为 JSON 对象；异常写入可能是被双重编码的 JSON 字符串 */
  request_body: Record<string, unknown> | string | null;
  response_body: string;
  // 词元用量
  token_input_tokens?: number | null;
  token_output_tokens?: number | null;
  token_cache_creation_input_tokens?: number | null;
  token_cache_read_input_tokens?: number | null;
  token_thinking_tokens?: number | null;
  token_total_tokens?: number | null;
  token_raw_usage?: Record<string, unknown> | null;
}

/** 通用分页结果 */
export interface PaginatedResult<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
}

/** 日志列表筛选条件 */
export interface LogFilters {
  startTime?: string;
  endTime?: string;
  sessionId?: string;
  userId?: string;
  accessPointId?: string;
  statusCode?: number | null;
  providerId?: string;
  accountId?: string;
  /** 是否中断：'true' = 已中断，'false' = 未中断，undefined = 不限 */
  isInterrupted?: string;
}
