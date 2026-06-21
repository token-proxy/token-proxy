import { type ReactNode, useCallback, useState } from 'react';
import { IconChevronRight } from '@douyinfe/semi-icons';

interface AccordionSectionProps {
  /** 手风琴标题（替代原 SectionHeading 的 children） */
  title: ReactNode;
  children: ReactNode;
  /** 默认展开，设为 false 则默认收起 */
  defaultExpanded?: boolean;
}

/**
 * 手风琴折叠区块
 *
 * 提供可点击标题 + 内容折叠/展开的手风琴交互。
 * 各 section 内部自管理展开状态，互不影响。
 */
/**
 * AccordionSection - 手风琴折叠区块组件
 *
 * 提供可点击标题 + 内容折叠/展开的手风琴交互。
 * 各 section 内部自管理展开状态，互不影响。
 */
export default function AccordionSection({
  title,
  children,
  defaultExpanded = true,
}: AccordionSectionProps): ReactNode {
  const [expanded, setExpanded] = useState(defaultExpanded);

  const toggle = useCallback(() => {
    setExpanded((prev) => !prev);
  }, []);

  return (
    <div className="accordion-section">
      <div
        className="accordion-header"
        onClick={toggle}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            toggle();
          }
        }}
      >
        <IconChevronRight
          size="small"
          style={{
            transition: 'transform 0.2s ease',
            transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)',
            marginRight: 8,
            color: 'var(--semi-color-text-2)',
            flexShrink: 0,
          }}
        />
        <span className="accordion-title">{title}</span>
      </div>
      <div className="accordion-body" style={{ display: expanded ? 'block' : 'none' }}>
        {children}
      </div>
    </div>
  );
}
