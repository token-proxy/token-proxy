import { type ReactNode } from 'react';
import { Descriptions } from '@douyinfe/semi-ui';
import CollapsibleCard from '@components/common/CollapsibleCard';

interface ContextManagementSectionProps {
  contextManagement: unknown;
}

/**
 * ContextManagementSection - 上下文管理展示区块
 *
 * 以折叠卡片形式展示请求中的 context_management 配置（支持字符串 JSON 自动解析）。
 */
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
