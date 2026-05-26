import { useState, type ReactNode } from 'react';
import { Button, Card } from '@douyinfe/semi-ui';
import {
  IconCopy,
  IconChevronUp,
  IconChevronDown,
} from '@douyinfe/semi-icons';

interface CollapsibleCardProps {
  title: string;
  defaultCollapsed?: boolean;
  copyText?: string;
  children: ReactNode;
  /** Card body style */
  bodyStyle?: React.CSSProperties;
  /** Card style */
  style?: React.CSSProperties;
}

/**
 * 可折叠 Card 组件
 *
 * - 标题栏左侧：折叠切换图标
 * - 标题栏右侧：复制按钮（可选）
 * - 标题栏 sticky 黏滞：当页面滚动时标题栏固定在顶部，直到 Card 完全滚出视图
 */
export default function CollapsibleCard({
  title,
  defaultCollapsed = false,
  copyText,
  children,
  bodyStyle,
  style,
}: CollapsibleCardProps): ReactNode {
  const [collapsed, setCollapsed] = useState(defaultCollapsed);
  const [copying, setCopying] = useState(false);

  const handleCopy = async () => {
    if (!copyText) return;
    setCopying(true);
    try {
      await navigator.clipboard.writeText(copyText);
    } finally {
      setCopying(false);
    }
  };

  return (
    <div className="collapsible-card-sticky">
    <Card
      style={style}
      bodyStyle={{
        display: collapsed ? 'none' : undefined,
        padding: bodyStyle?.padding ?? '0 24px 20px',
        ...bodyStyle,
      }}
      title={
        <div
          className="collapsible-card-header"
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <span
              className="collapsible-card-toggle"
              role="button"
              tabIndex={0}
              style={{
                cursor: 'pointer',
                display: 'inline-flex',
                alignItems: 'center',
                color: 'var(--semi-color-text-2)',
              }}
              onClick={(e) => {
                e.stopPropagation();
                setCollapsed(!collapsed);
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  setCollapsed(!collapsed);
                }
              }}
            >
              {collapsed ? <IconChevronDown /> : <IconChevronUp />}
            </span>
            {title}
          </span>
          {copyText && (
            <Button
              icon={<IconCopy />}
              size="small"
              type="tertiary"
              loading={copying}
              onClick={(e) => {
                e.stopPropagation();
                handleCopy();
              }}
            >
              复制
            </Button>
          )}
        </div>
      }
    >
      {children}
    </Card>
    </div>
  );
}
