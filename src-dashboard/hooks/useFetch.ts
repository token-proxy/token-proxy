//! 通用数据获取 Hook
//!
//! 封装 fetch-on-mount 模式，提供 loading / error / refetch 状态管理。
//! 所有 setState 在异步回调中执行，卸载后不再更新状态，防止内存泄漏。

import { useEffect, useMemo, useRef, useState } from 'react';

/**
 * useFetch 返回值
 */
export interface UseFetchResult<T> {
  /** 获取到的数据，初始为 null */
  data: T | null;
  /** 是否正在加载 */
  loading: boolean;
  /** 错误信息 */
  error: string | null;
  /** 手动重新获取 */
  refetch: () => void;
}

/**
 * 安全的数据获取 Hook
 *
 * 封装 fetch-on-mount 模式，所有 setState 在异步回调中执行。
 * loading 初始为 true，数据到达后设为 false，避免在 effect 同步体中调用 setState。
 *
 * @param fetcher - 异步获取函数，返回 Promise<T>
 * @param deps - 当依赖变化时重新执行 fetcher
 */
export function useFetch<T>(
  fetcher: () => Promise<T>,
  deps: React.DependencyList,
): UseFetchResult<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const execute = useMemo(
    () => () => {
      fetcher()
        .then((result) => {
          if (mountedRef.current) setData(result);
        })
        .catch((err) => {
          if (mountedRef.current) {
            setError(err instanceof Error ? err.message : String(err));
          }
        })
        .finally(() => {
          if (mountedRef.current) setLoading(false);
        });
    },
    // deps 由调用方动态传入，ESLint 无法静态验证依赖完整性
    // eslint-disable-next-line react-hooks/use-memo, react-hooks/exhaustive-deps
    deps,
  );

  useEffect(() => {
    execute();
  }, [execute]);

  return { data, loading, error, refetch: execute };
}
