import { type ReactNode } from 'react';
import AccordionSection from './AccordionSection';
import { extractContentBlocks } from './utils';
import MessageBlock from './MessageBlock';

interface MessagesSectionProps {
  messages: Array<Record<string, unknown>> | undefined;
}

export default function MessagesSection({
  messages,
}: MessagesSectionProps): ReactNode {
  if (!messages || messages.length === 0) return null;

  // 统计所有 content block 总数
  const totalBlocks = messages.reduce((sum, msg) => {
    return sum + extractContentBlocks((msg as Record<string, unknown>).content).length;
  }, 0);

  return (
    <AccordionSection
      title={`消息（共 ${messages.length} 条，${totalBlocks} 个内容块）`}
    >
      {messages.map((msg, msgIdx) => {
        const role = String((msg as Record<string, unknown>).role ?? 'unknown');
        const contentBlocks = extractContentBlocks(
          (msg as Record<string, unknown>).content,
        );
        return contentBlocks.map((block, bi) => (
          <MessageBlock
            key={`${msgIdx}-${bi}`}
            block={block}
            role={role}
          />
        ));
      })}
    </AccordionSection>
  );
}
