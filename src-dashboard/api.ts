/**
 * API 请求封装模块
 *
 * 提供 HTTP 请求的基础封装，包含 JWT 自动刷新机制（双层防御：请求前体检 + 401 兜底）、
 * 并发去重、认证失败重定向等功能。
 */

import type {
  HeatmapResponse,
  KpiResponse,
  QualityResponse,
  TimeRangeQuery,
  TopAccessPointsResponse,
  TopModelsResponse,
  UsageTrendsResponse,
} from './types/dashboard';
import type { AuditLogItem, AuditLogFilters } from './types/auditLog';
import type { PaginatedResult } from './types/log';
import type { LogStatsResponse, Settings, UpdateSettingsRequest } from './types/settings';

// access_token 距离过期不足该阈值时, 请求前主动刷新（秒）
const REFRESH_THRESHOLD_SEC = 300;

// 并发去重：所有需要刷新的请求 await 同一个 Promise, 避免后端 Refresh Token Rotation 互相吊销
let refreshing: Promise<string> | null = null;

/** 清除本地认证信息并跳转到登录页 */
function clearAuthAndRedirect(): void {
  localStorage.removeItem('access_token');
  localStorage.removeItem('refresh_token');
  localStorage.removeItem('username');
  localStorage.removeItem('display_name');
  window.location.href = '/login';
}

/** 本地解码 JWT payload, 不验签 (前端无密钥), 仅用于读取 exp 字段 */
function getTokenExp(token: string): number | null {
  try {
    const payload = token.split('.')[1];
    if (!payload) return null;
    const normalized = payload.replace(/-/g, '+').replace(/_/g, '/');
    const padded = normalized + '='.repeat((4 - (normalized.length % 4)) % 4);
    const json = JSON.parse(atob(padded));
    return typeof json.exp === 'number' ? json.exp : null;
  } catch {
    return null;
  }
}

/** 执行 refresh token 刷新，更新本地存储并返回新 access_token */
async function doRefresh(): Promise<string> {
  const refreshToken = localStorage.getItem('refresh_token');
  if (!refreshToken) {
    throw new Error('缺少 refresh_token');
  }

  const res = await fetch('/api/tokens:refresh', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ refresh_token: refreshToken }),
  });

  if (!res.ok) {
    throw new Error(`refresh 失败 (${res.status})`);
  }

  const data = await res.json();
  localStorage.setItem('access_token', data.access_token);
  localStorage.setItem('refresh_token', data.refresh_token);
  if (data.username) localStorage.setItem('username', data.username);
  if (data.display_name) localStorage.setItem('display_name', data.display_name);
  return data.access_token;
}

/**
 * 执行刷新：存在进行中的刷新请求时复用同一个 Promise 防止并发刷新互相吊销
 */
function scheduleRefresh(): Promise<string> {
  if (!refreshing) {
    refreshing = doRefresh().finally(() => {
      refreshing = null;
    });
  }
  return refreshing;
}

// 请求前体检：必要时主动刷新, 返回最新的 access_token
/** 请求前体检：必要时主动刷新, 返回最新的 access_token */
async function ensureFreshToken(): Promise<string | null> {
  // 已有 refresh 进行中, 直接 await 同一个 Promise
  if (refreshing) return refreshing;

  const token = localStorage.getItem('access_token');
  if (!token) return null;

  const exp = getTokenExp(token);
  if (!exp) return token;

  const now = Math.floor(Date.now() / 1000);
  if (exp - now > REFRESH_THRESHOLD_SEC) return token;

  try {
    return await scheduleRefresh();
  } catch {
    return token;
  }
}

/** 构建请求头：自动注入 Content-Type 和 Authorization */
async function buildHeaders(extra?: HeadersInit): Promise<HeadersInit> {
  const token = await ensureFreshToken();
  return {
    'Content-Type': 'application/json',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...(extra ?? {}),
  };
}

/**
 * 通用请求方法
 *
 * 自动注入认证头，401 时触发一次刷新重试。
 * 请求失败时输出 console.error 并 throw 错误。
 */
async function request<T>(url: string, options: RequestInit = {}, retried = false): Promise<T> {
  const headers = await buildHeaders(options.headers);
  const res = await fetch(url, { ...options, headers });

  // 401 兜底：体检漏判（如时钟漂移、服务端密钥轮换）时再尝试刷新一次
  if (res.status === 401 && !retried) {
    try {
      await scheduleRefresh();
      return request<T>(url, options, true);
    } catch {
      clearAuthAndRedirect();
      throw new Error('登录已过期');
    }
  }

  if (!res.ok) {
    const method = (options.method ?? 'GET').toUpperCase();
    const body = await res.json().catch(() => ({}));
    const errorMsg = body.error || `请求失败 (${res.status})`;
    console.error(`[API] ${method} ${url} 失败: ${res.status} - ${errorMsg}`, {
      request_id: body.request_id,
    });
    throw new Error(errorMsg);
  }

  // DELETE 等场景可能无响应体
  if (res.status === 204) return undefined as T;
  return res.json();
}

/** HTTP 请求封装，提供 get/post/put/delete 方法 */
const api = {
  /** GET 请求 */
  get<T>(url: string): Promise<T> {
    return request<T>(url);
  },

  /** POST 请求 */
  post<T>(url: string, body: unknown): Promise<T> {
    return request<T>(url, {
      method: 'POST',
      body: JSON.stringify(body),
    });
  },

  /** PUT 请求 */
  put<T>(url: string, body: unknown): Promise<T> {
    return request<T>(url, {
      method: 'PUT',
      body: JSON.stringify(body),
    });
  },

  /** DELETE 请求 */
  async delete(url: string): Promise<void> {
    await request<void>(url, { method: 'DELETE' });
  },
};

export default api;

// ─── Dashboard 数据洞察 API ───

/**
 * 将 `TimeRangeQuery` 序列化为 URL query string。
 *
 * - `range` 总是包含
 * - 仅 `custom` 模式包含 `start` / `end`（按需附加）
 */
function buildDashboardQuery(q: TimeRangeQuery): string {
  const params = new URLSearchParams({ range: q.range });
  if (q.range === 'custom') {
    if (q.start) params.set('start', q.start);
    if (q.end) params.set('end', q.end);
  }
  return params.toString();
}

/**
 * Dashboard 数据洞察 API 集合。
 *
 * 所有方法接受统一的 `TimeRangeQuery`（热力图除外，固定 365 天），对应后端 5 个聚合端点：
 * - `/api/getting-started/kpi` — 4 张 KPI 卡（含内嵌 sparkline）
 * - `/api/getting-started/heatmap` — 近 1 年活跃度热力图
 * - `/api/getting-started/top-models` — 用户视角模型 Top 8
 * - `/api/getting-started/top-access-points` — 用户视角接入点 Top 5
 * - `/api/getting-started/quality` — 用户视角调用质量指标
 * - `/api/getting-started/usage-trends` — 用户视角用量趋势
 */
export const dashboardApi = {
  /** 获取 4 张 KPI 卡数据 + 内嵌 sparkline 时间序列 */
  getKpi(q: TimeRangeQuery): Promise<KpiResponse> {
    return api.get<KpiResponse>(`/api/getting-started/kpi?${buildDashboardQuery(q)}`);
  },
  /**
   * 获取近 1 年用量热力图（固定 365 天，不受时间范围选择器影响）。
   *
   * @param tz IANA 时区名，建议传 `Intl.DateTimeFormat().resolvedOptions().timeZone`
   */
  getHeatmap(tz: string): Promise<HeatmapResponse> {
    return api.get<HeatmapResponse>(`/api/getting-started/heatmap?tz=${encodeURIComponent(tz)}`);
  },
  /** 获取用户视角模型 Top 8（按 total_tokens 降序） */
  getTopModels(q: TimeRangeQuery): Promise<TopModelsResponse> {
    return api.get<TopModelsResponse>(`/api/getting-started/top-models?${buildDashboardQuery(q)}`);
  },
  /** 获取用户视角接入点 Top 5（按 total_tokens 降序） */
  getTopAccessPoints(q: TimeRangeQuery): Promise<TopAccessPointsResponse> {
    return api.get<TopAccessPointsResponse>(
      `/api/getting-started/top-access-points?${buildDashboardQuery(q)}`,
    );
  },
  /** 获取用户视角调用质量指标（成功率 / 错误率 / 中断率 / 平均与 P95 耗时） */
  getQuality(q: TimeRangeQuery): Promise<QualityResponse> {
    return api.get<QualityResponse>(`/api/getting-started/quality?${buildDashboardQuery(q)}`);
  },
  /** 获取用户视角用量趋势（请求数 + 词元 5 维度时间序列） */
  getUsageTrends(q: TimeRangeQuery): Promise<UsageTrendsResponse> {
    return api.get<UsageTrendsResponse>(
      `/api/getting-started/usage-trends?${buildDashboardQuery(q)}`,
    );
  },
};

// ─── 审计日志 API ───

/**
 * 将对象构建为 URL 查询字符串，支持数组字段转逗号分隔。
 *
 * 与 `utils/query.ts` 中的 `buildQueryString` 类似，
 * 但额外支持 `string[]` 类型字段，自动以逗号连接。
 */
function buildAuditLogQuery(params: Record<string, unknown>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value === undefined || value === null || value === '') continue;
    if (Array.isArray(value)) {
      if (value.length > 0) search.set(key, value.join(','));
    } else {
      search.set(key, String(value));
    }
  }
  return search.toString();
}

/**
 * 审计日志 API 集合。
 *
 * 提供审计日志的分页查询，对应后端 `GET /api/audit-logs` 端点。
 */
export const auditLogApi = {
  /** 分页查询审计日志 */
  list(
    page: number,
    pageSize: number,
    filters: AuditLogFilters,
  ): Promise<PaginatedResult<AuditLogItem>> {
    const qs = buildAuditLogQuery({
      page,
      page_size: pageSize,
      start_time: filters.startTime,
      end_time: filters.endTime,
      action: filters.actions,
      entity_type: filters.entityTypes,
      operator_id: filters.operatorId,
      operator_type: filters.operatorType,
    });
    return api.get<PaginatedResult<AuditLogItem>>(`/api/audit-logs?${qs}`);
  },
};

// ─── 系统设置 API ───

/**
 * 系统设置 API 集合。
 *
 * 提供日志分区统计查看和系统设置读写功能，对应后端
 * - `GET /api/settings/log-stats` — 日志分区统计信息
 * - `GET /api/settings` — 获取系统设置
 * - `PUT /api/settings` — 更新系统设置
 */
export const settingsApi = {
  /** 获取日志分区统计信息 */
  getLogStats(): Promise<LogStatsResponse> {
    return api.get<LogStatsResponse>('/api/settings/log-stats');
  },

  /** 获取系统设置 */
  getSettings(): Promise<Settings> {
    return api.get<Settings>('/api/settings');
  },

  /** 更新系统设置 */
  updateSettings(values: UpdateSettingsRequest): Promise<Settings> {
    return api.put<Settings>('/api/settings', values);
  },

  /** 删除指定月份的日志分区（如 2026-01） */
  deleteMonthLogs(yearMonth: string): Promise<{ deleted: string[]; message: string }> {
    return request<{ deleted: string[]; message: string }>(`/api/settings/logs/${yearMonth}`, {
      method: 'DELETE',
    });
  },
};
