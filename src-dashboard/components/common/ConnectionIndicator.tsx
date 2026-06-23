import type { ReactNode } from 'react';
import type { SseStatus } from '../../hooks/useLogEvents';

/** ConnectionIndicator 组件 Props */
interface ConnectionIndicatorProps {
  /** SSE 连接状态 */
  status: SseStatus;
}

/** 各状态的视觉配置 */
const STATUS_CONFIG: Record<SseStatus, { color: string; label: string }> = {
  connecting: { color: 'var(--semi-color-warning)', label: '连接中...' },
  connected: { color: 'var(--semi-color-success)', label: '实时推送已连接' },
  disconnected: { color: 'var(--semi-color-disabled)', label: '未连接' },
  error: { color: 'var(--semi-color-danger)', label: '连接错误' },
};

/**
 * SSE 实时推送连接状态指示器
 *
 * 以彩色小圆点 + 文字标签展示当前 SSE 连接状态，
 * 放在手动刷新按钮旁边，让用户感知实时推送是否正常工作。
 */
export default function ConnectionIndicator({ status }: ConnectionIndicatorProps): ReactNode {
  const config = STATUS_CONFIG[status];
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 4,
        fontSize: 12,
        color: 'var(--semi-color-text-2)',
      }}
      title={config.label}
    >
      <span
        style={{
          display: 'inline-block',
          width: 8,
          height: 8,
          borderRadius: '50%',
          background: config.color,
          transition: 'background 0.3s',
        }}
      />
      {config.label}
    </span>
  );
}
