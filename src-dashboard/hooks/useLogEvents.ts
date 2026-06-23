//! SSE 实时日志事件消费 Hook
//!
//! 封装 EventSource 连接管理、JWT token 传递、自动重连、
//! 页面可见性暂停/恢复等逻辑。

import { useCallback, useEffect, useRef, useState } from 'react';

/** SSE 推送的新日志事件（对应后端 NewLogEvent DTO） */
export interface NewLogEvent {
  log_id: string;
  timestamp: string;
  session_id: string;
  api_type: string;
  user_id: string;
  access_point_id: string;
}

/** useLogEvents 连接状态 */
export type SseStatus = 'connecting' | 'connected' | 'disconnected' | 'error';

/** useLogEvents 返回值 */
export interface UseLogEventsResult {
  /** SSE 连接状态 */
  status: SseStatus;
  /** 最新接收到的事件 */
  lastEvent: NewLogEvent | null;
  /**
   * 注册页面从不可见恢复时应执行的全量刷新回调
   *
   * 页面隐藏期间可能错过了大量 SSE 事件，恢复时通过此回调通知调用方重新加载数据。
   */
  onVisibilityRecover: (callback: () => void) => void;
}

/**
 * 订阅后端 SSE 实时日志事件
 *
 * 自动管理 EventSource 生命周期：
 * - 通过 URL query 参数传递 JWT token（EventSource 不支持自定义 header）
 * - 连接断开后自动重试（5 秒延迟）
 * - 页面隐藏时暂停处理，恢复可见时通知调用方全量刷新
 *
 * 注意：EventSource 无法读取 HTTP 响应状态码，因此 JWT 过期时无法自动触发刷新。
 * 需要依赖 api.ts 中的请求前 JWT 过期体检机制。
 */
export function useLogEvents(): UseLogEventsResult {
  const [status, setStatus] = useState<SseStatus>('disconnected');
  const [lastEvent, setLastEvent] = useState<NewLogEvent | null>(null);
  const esRef = useRef<EventSource | null>(null);
  const visibilityCallbackRef = useRef<(() => void) | null>(null);
  const wasHiddenRef = useRef(false);

  // ─── 注册页面恢复可见时的全量刷新回调 ───

  const onVisibilityRecover = useCallback((callback: () => void) => {
    visibilityCallbackRef.current = callback;
  }, []);

  // ─── EventSource 连接管理 ───

  useEffect(() => {
    let cancelled = false;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

    function connect() {
      const token = localStorage.getItem('access_token');
      if (!token) {
        setStatus('error');
        return;
      }

      setStatus('connecting');

      // EventSource 不支持自定义 header，通过 URL query 参数传递 JWT
      const url = `/api/logs/events?token=${encodeURIComponent(token)}`;
      const es = new EventSource(url);
      esRef.current = es;

      es.onopen = () => {
        if (!cancelled) setStatus('connected');
      };

      es.onmessage = (event) => {
        if (cancelled) return;
        try {
          const parsed: NewLogEvent = JSON.parse(event.data);
          setLastEvent(parsed);
        } catch {
          // 忽略解析失败的事件
        }
      };

      es.onerror = () => {
        if (cancelled) return;
        es.close();
        setStatus('error');
        // 延迟 5 秒后重连，等待可能的 token 刷新
        reconnectTimer = setTimeout(() => {
          if (!cancelled) {
            connect();
          }
        }, 5000);
      };
    }

    connect();

    return () => {
      cancelled = true;
      if (reconnectTimer !== null) clearTimeout(reconnectTimer);
      esRef.current?.close();
      esRef.current = null;
    };
  }, []);

  // ─── 页面可见性管理 ───

  useEffect(() => {
    function handleVisibilityChange() {
      if (document.hidden) {
        wasHiddenRef.current = true;
      } else if (wasHiddenRef.current) {
        wasHiddenRef.current = false;
        // 页面恢复可见，通知调用方全量刷新
        visibilityCallbackRef.current?.();
      }
    }

    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, []);

  return { status, lastEvent, onVisibilityRecover };
}
