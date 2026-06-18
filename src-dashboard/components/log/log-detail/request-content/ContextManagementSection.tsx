import { type ReactNode } from 'react';
import { Descriptions } from '@douyinfe/semi-ui';
import CollapsibleCard from '@components/common/CollapsibleCard';

interface ContextManagementSectionProps {
  contextManagement: unknown;
}

export default function ContextManagementSection({
  contextManagement,
}: ContextManagementSectionProps): ReactNode {
  if (contextManagement == null) return null;

  const cmObj: Record<string, unknown> =
    typeof contextManagement === 'string'
      ? (() => {
        try {
          return JSON.parse(contextManagement);
        } catch {
          return {};
        }
      })()
      : (contextManagement as Record<string, unknown>);

  const entries = Object.entries(cmObj);
  if (entries.length === 0) return null;

  return (
    <CollapsibleCard title="上下文管理" defaultCollapsed>
      <Descriptions
        row
        size="small"
        data={entries.map(([k, v]) => ({
          key: k,
          value: typeof v === 'string' ? v : JSON.stringify(v),
        }))}
      />
    </CollapsibleCard>
  );
}
