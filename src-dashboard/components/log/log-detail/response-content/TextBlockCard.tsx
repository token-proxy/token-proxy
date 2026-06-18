import { type ReactNode } from 'react';
import { Collapse, Tag } from '@douyinfe/semi-ui';
import ExpandableContentBlock from '@components/common/ExpandableContentBlock';
import { type ContentBlockInfo } from '../../../../utils/parseLogs.ts';

interface TextBlockCardProps {
  block: ContentBlockInfo;
  itemKey: string;
}

/** 块 C: 助手回复 */
export default function TextBlockCard({
  block,
  itemKey,
}: TextBlockCardProps): ReactNode | null {
  if (block.block_type !== 'text' || !block.text) return null;

  return (
    <Collapse.Panel
      itemKey={itemKey}
      header={
        <div style={{display: 'flex', alignItems: 'center', gap: 8}}>
          <Tag color="green" size="small">助手回复</Tag>
        </div>
      }
    >
      <ExpandableContentBlock
        content={block.text}
        collapseLabel="收起回复内容"
      />
    </Collapse.Panel>
  );
}
