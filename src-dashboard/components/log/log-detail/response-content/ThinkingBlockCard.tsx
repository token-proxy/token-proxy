import { type ReactNode } from 'react';
import { Collapse, Tag } from '@douyinfe/semi-ui';
import ExpandableContentBlock from '@components/common/ExpandableContentBlock';
import { type ContentBlockInfo } from '../../../../utils/parseLogs.ts';

interface ThinkingBlockCardProps {
  block: ContentBlockInfo;
  itemKey: string;
}

/** 块 B: 思考过程 */
export default function ThinkingBlockCard({
  block,
  itemKey,
}: ThinkingBlockCardProps): ReactNode | null {
  if (block.block_type !== 'thinking' || !block.thinking) return null;

  return (
    <Collapse.Panel
      itemKey={itemKey}
      header={
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <Tag color="blue" size="small">
            思考过程
          </Tag>
        </div>
      }
    >
      <ExpandableContentBlock
        content={block.thinking}
        defaultExpanded={false}
        collapseLabel="收起思考内容"
      />
    </Collapse.Panel>
  );
}
