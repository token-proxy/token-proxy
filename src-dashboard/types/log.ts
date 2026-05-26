export interface UserItem {
  id: string;
  username: string;
  display_name: string;
  status: string;
}

export interface AccessPointItem {
  id: string;
  name: string;
  short_code: string;
}

export interface SessionSummary {
  session_id: string;
  user_id?: string | null;
  access_point_id?: string | null;
  start_time: string;
  request_count: number;
  // Token 汇总
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_creation_input_tokens: number;
  total_cache_read_input_tokens: number;
  total_thinking_tokens: number;
  total_tokens: number;
}

export interface LogSummary {
  id: string;
  timestamp: string;
  session_id: string;
  user_id?: string | null;
  access_point_id?: string | null;
  model_original?: string | null;
  model_mapped?: string | null;
  status_code?: number | null;
  duration_ms?: number | null;
  conversation_source: string;
  agent_id?: string | null;
  raw_content_available: boolean;
  // Token 摘要
  token_input_tokens?: number | null;
  token_output_tokens?: number | null;
  token_total_tokens?: number | null;
  // 客户端信息
  client_name?: string | null;
  client_version?: string | null;
  client_channel?: string | null;
  client_platform?: string | null;
  // API 类型
  api_type?: string;
}

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

/** 会话事件，由前端 parseLogs.ts 从原始请求/响应体构建 */
export interface ConversationEvent {
  id: string;
  log_id: string;
  timestamp: string;
  request_index: number;
  event_index: number;
  source: string;
  role: string;
  event_type: string;
  agent_id?: string;
  agent_type?: string;
  tool_use_id?: string;
  tool_name?: string;
  title?: string;
  content?: string;
  thinking_content?: string;
  display_payload?: Record<string, unknown>;
  tool_result_content?: string;
}

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
  request_index: number;
  conversation_source: string;
  agent_id?: string | null;
  // 客户端信息
  client_name?: string | null;
  client_version?: string | null;
  client_channel?: string | null;
  client_platform?: string | null;
  // 请求 + 响应原始内容（前端自行解析）
  request_headers: Record<string, unknown> | null;
  request_body: Record<string, unknown> | null;
  response_body: string;
  // Token 用量
  token_input_tokens?: number | null;
  token_output_tokens?: number | null;
  token_cache_creation_input_tokens?: number | null;
  token_cache_read_input_tokens?: number | null;
  token_thinking_tokens?: number | null;
  token_total_tokens?: number | null;
  token_raw_usage?: Record<string, unknown> | null;
}

/** 会话原始内容项（前端基于此构建事件流） */
export interface SessionContentItem {
  log_id: string;
  request_index: number;
  timestamp: string;
  conversation_source: string;
  agent_id?: string | null;
  request_headers: Record<string, unknown>;
  request_body: Record<string, unknown>;
  response_body: string;
  token_usage?: TokenUsage | null;
}

export interface PaginatedResult<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
}

export interface SessionListFilters {
  startTime?: string;
  endTime?: string;
  userId?: string;
  accessPointId?: string;
}

export interface LogFilters {
  startTime?: string;
  endTime?: string;
  sessionId?: string;
  userId?: string;
  accessPointId?: string;
  statusCode?: number | null;
}
