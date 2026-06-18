import { type ReactNode } from 'react';
import { Collapse, Tag, Typography } from '@douyinfe/semi-ui';
import CodeHighlight from '@components/common/CodeHighlight';
import { type ContentBlockInfo } from '../../../../utils/parseLogs.ts';

const {Text} = Typography;

interface ToolUseBlockCardProps {
  block: ContentBlockInfo;
  itemKey: string;
}

/** 块 D: 工具调用 */
export default function ToolUseBlockCard({
  block,
  itemKey,
}: ToolUseBlockCardProps): ReactNode | null {
  if (block.block_type !== 'tool_use') return null;

  return (
    <Collapse.Panel
      itemKey={itemKey}
      header={
        <div style={{display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap'}}>
          <Tag color="purple" size="small">工具调用</Tag>
          {block.tool_name && (
            <Tag color="violet" size="small">{block.tool_name}</Tag>
          )}
        </div>
      }
    >
      {block.tool_use_id && (
        <div style={{marginBottom: 12}}>
          <Text size="small" type="secondary">调用 ID: </Text>
          <span className="monospace-text">{block.tool_use_id}</span>
        </div>
      )}
      {block.input && (
        <div>
          <Text size="small" type="secondary" style={{display: 'block', marginBottom: 4}}>
            参数:
          </Text>
          <CodeHighlight content={JSON.stringify(block.input, null, 2)} language="json"/>
        </div>
      )}
    </Collapse.Panel>
  );
}
