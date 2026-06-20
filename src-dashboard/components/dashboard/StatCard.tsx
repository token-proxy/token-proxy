import { Card, Spin, Tag, Typography } from '@douyinfe/semi-ui';
import { IconArrowDown, IconArrowUp, IconMinus } from '@douyinfe/semi-icons';
import type { ReactNode } from 'react';

const {Title, Text} = Typography;

interface ChangeIndicatorProps {
  value?: number;
  invert?: boolean;
}

function ChangeIndicator({value, invert}: ChangeIndicatorProps): ReactNode {
  if (value === undefined || value === null || Number.isNaN(value)) return null;

  const adjusted = invert ? -value : value;
  const color = adjusted > 0 ? 'green' : adjusted < 0 ? 'red' : 'grey';
  const Icon = adjusted > 0 ? IconArrowUp : adjusted < 0 ? IconArrowDown : IconMinus;
  const label = value > 0 ? `+${value}%` : value < 0 ? `${value}%` : '0%';

  return (
    <Tag
      color={color}
      style={{marginLeft: 8, display: 'inline-flex', alignItems: 'center', gap: 2}}
    >
      <Icon size="small"/>
      {label}
    </Tag>
  );
}

/** StatCard 组件 Props */
interface StatCardProps {
  title: string;
  value: string | number;
  change?: number;
  loading: boolean;
  invertChange?: boolean;
}

/**
 * StatCard - Dashboard 统计卡片组件
 *
 * 展示单个指标数值、变化趋势（上升/下降/持平）、加载态。
 */
export default function StatCard({
  title,
  value,
  change,
  loading,
  invertChange,
}: StatCardProps): ReactNode {
  return (
    <Card style={{minHeight: 140, backgroundColor: 'var(--semi-color-bg-0)'}}>
      <Text type="secondary" style={{fontSize: 14}}>
        {title}
      </Text>
      {loading ? (
        <div style={{marginTop: 20}}>
          <Spin size="small"/>
        </div>
      ) : (
        <>
          <Title heading={3} style={{marginTop: 8, marginBottom: 4}}>
            {value}
          </Title>
          <ChangeIndicator value={change} invert={invertChange}/>
        </>
      )}
    </Card>
  );
}
