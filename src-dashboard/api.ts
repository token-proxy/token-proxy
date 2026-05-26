// access_token 距离过期不足该阈值时, 请求前主动刷新（秒）
const REFRESH_THRESHOLD_SEC = 300;

// 并发去重：所有需要刷新的请求 await 同一个 Promise, 避免后端 Refresh Token Rotation 互相吊销
let refreshing: Promise<string> | null = null;

function clearAuthAndRedirect(): void {
  localStorage.removeItem('access_token');
  localStorage.removeItem('refresh_token');
  localStorage.removeItem('username');
  localStorage.removeItem('display_name');
  window.location.href = '/login';
}

// 本地解码 JWT payload, 不验签 (前端无密钥), 仅用于读取 exp 字段
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

async function doRefresh(): Promise<string> {
  const refreshToken = localStorage.getItem('refresh_token');
  if (!refreshToken) {
    throw new Error('缺少 refresh_token');
  }

  const res = await fetch('/api/auth/refresh', {
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

function scheduleRefresh(): Promise<string> {
  if (!refreshing) {
    refreshing = doRefresh().finally(() => {
      refreshing = null;
    });
  }
  return refreshing;
}

// 请求前体检：必要时主动刷新, 返回最新的 access_token
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

async function buildHeaders(extra?: HeadersInit): Promise<HeadersInit> {
  const token = await ensureFreshToken();
  return {
    'Content-Type': 'application/json',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...(extra ?? {}),
  };
}

async function request<T>(
  url: string,
  options: RequestInit = {},
  retried = false,
): Promise<T> {
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
    const body = await res.json().catch(() => ({}));
    throw new Error(body.error || `请求失败 (${res.status})`);
  }

  // DELETE 等场景可能无响应体
  if (res.status === 204) return undefined as T;
  return res.json();
}

const api = {
  get<T>(url: string): Promise<T> {
    return request<T>(url);
  },

  post<T>(url: string, body: unknown): Promise<T> {
    return request<T>(url, {
      method: 'POST',
      body: JSON.stringify(body),
    });
  },

  put<T>(url: string, body: unknown): Promise<T> {
    return request<T>(url, {
      method: 'PUT',
      body: JSON.stringify(body),
    });
  },

  async delete(url: string): Promise<void> {
    await request<void>(url, { method: 'DELETE' });
  },
};

export default api;
