/**
 * CacheHitCard - 缓存命中率 KPI 卡片（无 Sparkline）。
 *
 * 单独类型卡片：因为缓存命中率是比率而非计数，没有合适的时间序列序列。
 * 底部展示命中率的定义公式说明，占据与 KpiCard Sparkline 等高的空间以保持视觉对齐。
 */

import { Card, Skeleton } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import type { TrendBadge } from '../../types/dashboard';
import { ComparisonArrow } from './ComparisonArrow';

/** CacheHitCard 组件 Props */
interface CacheHitCardProps {
  /** 命中率（0.0 - 1.0）；null = 无可命中数据，展示 "—" */
  rate: number | null;
  /** 趋势徽章（基于命中率变化） */
  trend: TrendBadge;
  /** 命中率变化百分比；null = 无法计算 */
  changePct: number | null;
  /** 加载态：true 时渲染 Skeleton 占位 */
  loading?: boolean;
}

/**
 * 缓存命中率卡片。
 *
 * 布局自上而下：
 * 1. 标题 "缓存命中率"（13px 二级文字）
 * 2. 命中率百分比（36px 加粗）+ ComparisonArrow
 * 3. 公式说明 `cache_read / (input + cache_read)`（48px 高，与 KpiCard 的 Sparkline 等高）
 *
 * rate 为 null 时展示 `—` 占位，避免误传 0% 引起歧义。
 *
 * @example
 * <CacheHitCard rate={0.342} trend="up" changePct={3.5} />
 * <CacheHitCard rate={null} trend="empty" changePct={null} />
 */
export function CacheHitCard({
  rate,
  trend,
  changePct,
  loading = false,
}: CacheHitCardProps): ReactNode {
  if (loading) {
    return (
      <Card
        bordered={false}
        style={{
          height: 160,
          backgroundColor: 'var(--semi-color-bg-2)',
          borderRadius: 12,
        }}
        bodyStyle={{ padding: 20 }}
      >
        <Skeleton active placeholder={<Skeleton.Paragraph rows={3} />} loading={true} />
      </Card>
    );
  }

  // rate 为 null 时显示破折号占位
  const rateDisplay = rate == null ? '—' : `${(rate * 100).toFixed(1)}%`;

  return (
    <Card
      bordered={false}
      style={{
        height: 160,
        backgroundColor: 'var(--semi-color-bg-2)',
        borderRadius: 12,
      }}
      bodyStyle={{ padding: 20 }}
    >
      {/* 标题 */}
      <div
        style={{
          fontSize: 13,
          color: 'var(--semi-color-text-2)',
          marginBottom: 8,
        }}
      >
        缓存命中率
      </div>

      {/* 百分比 + 趋势箭头 */}
      <div
        style={{
          display: 'flex',
          alignItems: 'baseline',
          gap: 12,
          marginBottom: 12,
        }}
      >
        <span
          style={{
            fontSize: 36,
            fontWeight: 600,
            color: 'var(--semi-color-text-0)',
            lineHeight: 1,
          }}
        >
          {rateDisplay}
        </span>
        <ComparisonArrow trend={trend} changePct={changePct} />
      </div>

      {/* 公式说明（占据与 KpiCard Sparkline 等高的空间） */}
      <div
        style={{
          fontSize: 12,
          color: 'var(--semi-color-text-2)',
          fontStyle: 'italic',
          height: 48,
          display: 'flex',
          alignItems: 'center',
        }}
      >
        cache_read / (input + cache_read)
      </div>
    </Card>
  );
}
