import { Popconfirm, Tag } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';

/** StatusToggle 组件 Props */
interface StatusToggleProps {
  enabled: boolean;
  loading?: boolean;
  confirmTitle?: string;
  /** 确认气泡内的补充说明（如禁用后权限立即失效） */
  confirmContent?: string;
  onToggle: () => void;
}

/**
 * StatusToggle - 启用/禁用状态切换组件
 *
 * 展示当前状态的 Tag，点击后弹出确认气泡（Popconfirm），确认后触发切换。
 */
export default function StatusToggle({
  enabled,
  loading = false,
  confirmTitle,
  confirmContent,
  onToggle,
}: StatusToggleProps): ReactNode {
  const tag = (
    <Tag
      color={enabled ? 'green' : 'red'}
      style={{ cursor: loading ? 'not-allowed' : 'pointer', opacity: loading ? 0.5 : 1 }}
    >
      {enabled ? '启用' : '禁用'}
    </Tag>
  );

  if (loading) return tag;

  return (
    <Popconfirm
      title={confirmTitle ?? `确认${enabled ? '禁用' : '启用'}?`}
      content={confirmContent}
      onConfirm={onToggle}
      position="bottomRight"
    >
      {tag}
    </Popconfirm>
  );
}
