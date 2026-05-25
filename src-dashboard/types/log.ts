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
  first_message?: string | null;
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
  agent_type?: string | null;
  request_kind?: string | null;
  primary_tool_name?: string | null;
  message_preview?: string | null;
  message_full?: string | null;
  response_preview?: string | null;
  has_thinking: boolean;
  has_tool_use: boolean;
  raw_content_available: boolean;
  // Token 摘要
  token_input_tokens?: number | null;
  token_output_tokens?: number | null;
  token_total_tokens?: number | null;
  // 客户端信息
  parser_version?: string | null;
  client_name?: string | null;
  client_version?: string | null;
  client_channel?: string | null;
  client_platform?: string | null;
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

export interface ConversationEvent {
  id: string;
  log_id: string;
  session_id: string;
  timestamp: string;
  request_index: number;
  event_index: number;
  parent_event_id?: string | null;
  parent_tool_use_id?: string | null;
  source: string;
  role: string;
  event_type: string;
  agent_id?: string | null;
  agent_type?: string | null;
  tool_use_id?: string | null;
  tool_name?: string | null;
  title?: string | null;
  content?: string | null;
  content_preview?: string | null;
  thinking_content?: string | null;
  hidden_content?: Record<string, unknown> | null;
  display_payload?: Record<string, unknown> | null;
  confidence: number;
  // 新增字段
  content_type?: string | null;
  signature?: string | null;
  tool_result_content?: string | null;
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
  // 新增字段
  server_tool_usage?: Record<string, unknown> | null;
  cache_creation?: Record<string, unknown> | null;
}

/** 日志完整详情（用于独立详情页 /logs/:id） */
export interface LogDetailFull {
  id: string;
  timestamp: string;
  session_id: string;
  user_id?: string | null;
  access_point_id?: string | null;
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
  agent_type?: string | null;
  // 客户端信息
  parser_version?: string | null;
  client_name?: string | null;
  client_version?: string | null;
  client_channel?: string | null;
  client_platform?: string | null;
  // 请求内容
  request_headers: Record<string, unknown>;
  request_body: Record<string, unknown>;
  request_message_text?: string | null;
  // 响应内容
  response_body: string;
  response_assistant_text?: string | null;
  response_thinking_text?: string | null;
  // Token 用量
  token_input_tokens?: number | null;
  token_output_tokens?: number | null;
  token_cache_creation_input_tokens?: number | null;
  token_cache_read_input_tokens?: number | null;
  token_thinking_tokens?: number | null;
  token_total_tokens?: number | null;
  token_raw_usage?: Record<string, unknown> | null;
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
