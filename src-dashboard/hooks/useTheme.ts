import { createContext, createElement, type ReactNode, useContext, useEffect, useMemo, useState } from 'react';

export type ThemeMode = 'light' | 'dark' | 'system';
export type EffectiveTheme = 'light' | 'dark';

const STORAGE_KEY = 'theme_mode';

function getStoredMode(): ThemeMode {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === 'light' || stored === 'dark' || stored === 'system') {
    return stored;
  }
  return 'system';
}

function getSystemTheme(): EffectiveTheme {
  return window.matchMedia('(prefers-color-scheme: dark)').matches
    ? 'dark'
    : 'light';
}

function resolveEffectiveTheme(mode: ThemeMode): EffectiveTheme {
  return mode === 'system' ? getSystemTheme() : mode;
}

function applyTheme(effective: EffectiveTheme): void {
  if (effective === 'dark') {
    document.body.setAttribute('theme-mode', 'dark');
  } else {
    document.body.removeAttribute('theme-mode');
  }
}

interface ThemeContextValue {
  mode: ThemeMode;
  effectiveTheme: EffectiveTheme;
  setMode: (mode: ThemeMode) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({children}: { children: ReactNode }): ReactNode {
  const [mode, setMode] = useState<ThemeMode>(getStoredMode);
  const [, forceUpdate] = useState(0);

  const effectiveTheme = resolveEffectiveTheme(mode);

  useEffect(() => {
    applyTheme(effectiveTheme);
  }, [effectiveTheme]);

  useEffect(() => {
    if (mode !== 'system') return;
    const mql = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = () => forceUpdate((n) => n + 1);
    mql.addEventListener('change', handler);
    return () => mql.removeEventListener('change', handler);
  }, [mode]);

  useEffect(() => {
    const handler = (event: StorageEvent) => {
      if (event.key === STORAGE_KEY) {
        setMode(getStoredMode());
      }
    };
    window.addEventListener('storage', handler);
    return () => window.removeEventListener('storage', handler);
  }, []);

  const setThemeMode = (newMode: ThemeMode) => {
    localStorage.setItem(STORAGE_KEY, newMode);
    setMode(newMode);
  };

  const value = useMemo(
    () => ({mode, effectiveTheme, setMode: setThemeMode}),
    [mode, effectiveTheme],
  );

  return createElement(ThemeContext.Provider, {value}, children);
}

/**
 * useTheme - 主题管理 Hook
 *
 * @returns 当前主题模式、生效主题类型、设置主题方法
 */
export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return ctx;
}