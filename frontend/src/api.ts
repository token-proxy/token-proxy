const getHeaders = () => ({
  'Content-Type': 'application/json',
  'Authorization': `Bearer ${localStorage.getItem('access_token')}`,
});

function handleUnauthorized(res: Response): void {
  if (res.status === 401) {
    localStorage.removeItem('access_token');
    localStorage.removeItem('refresh_token');
    localStorage.removeItem('username');
    window.location.href = '/login';
  }
}

async function request<T>(url: string, options?: RequestInit): Promise<T> {
  const res = await fetch(url, options);
  handleUnauthorized(res);
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error(body.error || `请求失败 (${res.status})`);
  }
  return res.json();
}

const api = {
  get<T>(url: string): Promise<T> {
    return request<T>(url, { headers: getHeaders() });
  },

  post<T>(url: string, body: unknown): Promise<T> {
    return request<T>(url, {
      method: 'POST',
      headers: getHeaders(),
      body: JSON.stringify(body),
    });
  },

  put<T>(url: string, body: unknown): Promise<T> {
    return request<T>(url, {
      method: 'PUT',
      headers: getHeaders(),
      body: JSON.stringify(body),
    });
  },

  async delete(url: string): Promise<void> {
    const res = await fetch(url, { method: 'DELETE', headers: getHeaders() });
    handleUnauthorized(res);
    if (!res.ok) {
      const body = await res.json().catch(() => ({}));
      throw new Error(body.error || `请求失败 (${res.status})`);
    }
  },
};

export default api;