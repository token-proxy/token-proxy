import { Tag, Popconfirm } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';

interface StatusToggleProps {
  enabled: boolean;
  loading?: boolean;
  confirmTitle?: string;
  onToggle: () => void;
}

export default function StatusToggle({
  enabled,
  loading = false,
  confirmTitle,
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
      onConfirm={onToggle}
      position="bottomRight"
    >
      {tag}
    </Popconfirm>
  );
}
