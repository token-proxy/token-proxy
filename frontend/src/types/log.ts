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
