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

/** 会话汇总信息 */
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
  raw_content_available: boolean;
  // Token 摘要
  token_input_tokens?: number | null;
  token_output_tokens?: number | null;
  token_cache_creation_input_tokens?: number | null;
  token_cache_read_input_tokens?: number | null;
  token_thinking_tokens?: number | null;
  token_total_tokens?: number | null;
  // 客户端信息
  client_name?: string | null;
  client_version?: string | null;
  client_channel?: string | null;
  client_platform?: string | null;
  // API 类型
  api_type?: string;
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

/** 会话事件，由前端 parseLogs.ts 从原始请求/响应体构建 */
export interface ConversationEvent {
  id: string;
  log_id: string;
  timestamp: string;
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

/** Token 用量记录 */
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
  client_name?: string | null;
  client_version?: string | null;
  client_channel?: string | null;
  client_platform?: string | null;
  // 请求 + 响应原始内容（前端自行解析）
  request_headers: Record<string, unknown> | null;
  response_headers: Record<string, unknown> | null;
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
  timestamp: string;
  conversation_source: string;
  agent_id?: string | null;
  request_headers: Record<string, unknown>;
  request_body: Record<string, unknown>;
  response_body: string;
  token_usage?: TokenUsage | null;
}

/** 通用分页结果 */
export interface PaginatedResult<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
}

/** 会话列表筛选条件 */
export interface SessionListFilters {
  startTime?: string;
  endTime?: string;
  userId?: string;
  accessPointId?: string;
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

/** 轮次级 Token 用量汇总 */
export interface TurnTokenSummary {
  inputTokens: number;
  outputTokens: number;
  cacheCreationTokens: number;
  cacheReadTokens: number;
  thinkingTokens: number;
  totalTokens: number;
}

/**
 * 轮次内的事件块
 *
 * 使用联合类型表示不同类型的对话内容块，
 * agent_call 通过 children 字段支持子代理递归嵌套。
 */
export type TurnBlock =
  | { type: 'thinking'; content: string; logId: string; timestamp: string }
  | {
      type: 'tool_use';
      toolName: string;
      input: Record<string, unknown>;
      logId: string;
      timestamp: string;
    }
  | {
      type: 'tool_result';
      toolUseId: string;
      content: string;
      isError: boolean;
      logId: string;
      timestamp: string;
    }
  | { type: 'assistant_message'; content: string; logId: string; timestamp: string }
  | {
      type: 'agent_call';
      agentType: string;
      children: TurnBlock[];
      logIds: string[];
      tokenSummary: TurnTokenSummary;
      logId: string;
      timestamp: string;
    };

/**
 * 对话轮次
 *
 * 从一个非 tool_result 的用户消息开始，到助手完成回答结束。
 * 一个轮次可能跨越多次 HTTP 请求（当存在 tool_use roundtrip 时）。
 */
export interface ConversationTurn {
  /** 轮次唯一标识 */
  id: string;
  /** 轮次序号（从 1 开始） */
  turnIndex: number;
  /** 轮次开始时间 */
  startTime: string;
  /** 轮次结束时间 */
  endTime: string;
  /** 该轮次的用户消息内容 */
  userMessage: string;
  /** 该轮次内的所有事件块（按时间排序） */
  blocks: TurnBlock[];
  /** 该轮次的 Token 汇总 */
  tokenSummary: TurnTokenSummary;
  /** 该轮次涉及的所有 log_id */
  logIds: string[];
}
