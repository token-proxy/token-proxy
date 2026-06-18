import { type ReactNode, useState } from 'react';
import { Tag } from '@douyinfe/semi-ui';
import AccordionSection from './AccordionSection';
import ToolDetail from './ToolDetail';

interface ToolsSectionProps {
  tools: Array<Record<string, unknown>> | undefined;
}

export default function ToolsSection({
  tools,
}: ToolsSectionProps): ReactNode {
  if (!tools || tools.length === 0) return null;

  const [expandedToolIdx, setExpandedToolIdx] = useState<number | null>(null);

  const handleTagClick = (idx: number) => {
    setExpandedToolIdx((prev) => (prev === idx ? null : idx));
  };

  return (
    <AccordionSection
      title={`工具定义（共 ${tools.length} 个）`}
      defaultExpanded={false}
    >
      <div style={{display: 'flex', flexWrap: 'wrap', gap: 8, marginBottom: 8}}>
        {tools.map((tool, idx) => {
          const name = String(tool.name ?? '');
          return (
            <Tag
              key={idx}
              size="small"
              color={expandedToolIdx === idx ? 'blue' : 'grey'}
              style={{cursor: 'pointer'}}
              onClick={() => handleTagClick(idx)}
            >
              {name}
            </Tag>
          );
        })}
      </div>
      {expandedToolIdx !== null && tools[expandedToolIdx] && (
        <ToolDetail tool={tools[expandedToolIdx]}/>
      )}
    </AccordionSection>
  );
}
