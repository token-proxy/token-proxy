import { type ReactNode, useState } from 'react';
import { Card } from '@douyinfe/semi-ui';
import { IconChevronDown, IconChevronUp } from '@douyinfe/semi-icons';
import type { CardProps } from '@douyinfe/semi-ui/lib/es/card';

interface CollapsibleCardProps extends CardProps {
  defaultCollapsed?: boolean;
}

/**
 * 可折叠 Card 组件
 *
 * - 点击标题栏任意区域切换折叠状态
 * - 交互元素（按钮、Switch 等）上的点击不会触发折叠
 * - 图标与标题内容垂直居中对齐
 */
export default function CollapsibleCard({
  defaultCollapsed = false,
  ...props
}: CollapsibleCardProps): ReactNode {
  const [collapsed, setCollapsed] = useState(defaultCollapsed);

  const handleHeaderClick = (e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    // 不拦截交互元素自身的点击行为
    if (
      target.closest(
        'button, input, select, a, label, [role="button"], [role="switch"], [role="checkbox"], .semi-switch, .semi-button',
      )
    ) {
      return;
    }
    if (target.closest('.semi-card-header')) {
      setCollapsed((prev) => !prev);
    }
  };

  const collapsedIcon = collapsed ? <IconChevronDown /> : <IconChevronUp />;

  return (
    <div onClick={handleHeaderClick}>
      <Card
        {...props}
        style={{
          ...props.style,
          overflow: 'visible', // 覆盖 Semi Card 默认 overflow: hidden，释放后代 sticky 元素
        }}
        headerStyle={{
          ...props.headerStyle,
          cursor: 'pointer',
        }}
        bodyStyle={{
          ...props.bodyStyle,
          display: collapsed ? 'none' : undefined,
        }}
        title={
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
            {collapsedIcon}
            {props.title}
          </span>
        }
      >
        {props.children}
      </Card>
    </div>
  );
}
