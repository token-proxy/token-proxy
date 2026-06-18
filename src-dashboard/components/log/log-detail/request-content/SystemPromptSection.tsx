import { type ReactNode } from 'react';
import { Tag } from '@douyinfe/semi-ui';
import ExpandableContentBlock from '@components/common/ExpandableContentBlock';
import AccordionSection from './AccordionSection';

interface SystemPromptSectionProps {
  system: Array<Record<string, unknown>> | undefined;
}

export default function SystemPromptSection({
  system,
}: SystemPromptSectionProps): ReactNode {
  const textBlocks = (system ?? []).filter(
    (block) => block.type === 'text' && typeof block.text === 'string',
  );

  if (textBlocks.length === 0) return null;

  return (
    <AccordionSection
      title={`系统提示词（共 ${textBlocks.length} 个文本块）`}
      defaultExpanded={false}
    >
      {textBlocks.map((block, idx) => (
        <SystemPromptBlock key={idx} block={block} index={idx}/>
      ))}
    </AccordionSection>
  );
}

function SystemPromptBlock({block}: {
  block: Record<string, unknown>;
  index: number;
}): ReactNode {
  const text = block.text as string;
  if (!text) return null;

  return (
    <div
      style={{
        marginBottom: 12,
        padding: 12,
        border: '1px solid var(--semi-color-border)',
        borderRadius: 6,
      }}
    >
      <div style={{marginBottom: 8}}>
        <Tag size="small" type="light">
          文本
        </Tag>
      </div>
      <ExpandableContentBlock
        content={text}
        collapseLabel="收起文本块"
      />
    </div>
  );
}
