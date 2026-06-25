/**
 * KpiCard - 含内嵌 Sparkline 的 KPI 卡片。
 *
 * 用于请求数 / 词元量两张主 KPI 卡，统一视觉与交互。
 * 无 sparkline 的比率类指标请使用独立的指标卡片组件。
 */

import { Card, Skeleton } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import type { TrendBadge } from '../../types/dashboard';
import { ComparisonArrow } from './ComparisonArrow';
import { Sparkline } from './Sparkline';

/** KpiCard 组件 Props */
interface KpiCardProps {
  /** 卡片标题（如 "我的请求数"） */
  title: string;
  /** 当前窗口值（KpiTrendItem.current） */
  value: number;
  /** 数字格式化器，默认使用中文千分位 */
  format?: (n: number) => string;
  /** 趋势徽章（来自 KpiTrendItem.trend） */
  trend: TrendBadge;
  /** 百分比变化（来自 KpiTrendItem.change_pct） */
  changePct: number | null;
  /** Sparkline 数据点序列 */
  sparklineData: number[];
  /** 加载态：true 时渲染 Skeleton 占位 */
  loading?: boolean;
}

/**
 * KPI 卡片（含内嵌 Sparkline）。
 *
 * 布局自上而下：
 * 1. 标题（14px，letter-spacing 增强可读性）
 * 2. 大数字（36px 加粗，tabular-nums 等宽数字）+ ComparisonArrow（baseline 对齐）
 * 3. Sparkline（48px 高的极简单色折线）
 *
 * 样式策略：
 * - 卡片背景使用 `--semi-color-bg-2`，去边框 + 12px 圆角，暗色优先
 * - 所有颜色经 Semi CSS 变量，明暗主题自动适配
 * - 整体高度固定 160px，与同列卡片视觉对齐
 *
 * @example
 * <KpiCard
 *   title="我的请求数"
 *   value={3847}
 *   trend="up"
 *   changePct={18.2}
 *   sparklineData={[1, 5, 3, 8, 6, 12, 9]}
 * />
 */
export function KpiCard({
  title,
  value,
  format = defaultFormat,
  trend,
  changePct,
  sparklineData,
  loading = false,
}: KpiCardProps): ReactNode {
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
          fontSize: 14,
          fontWeight: 500,
          letterSpacing: '0.02em',
          color: 'var(--semi-color-text-2)',
          marginBottom: 8,
        }}
      >
        {title}
      </div>

      {/* 大数字 + 趋势箭头 */}
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
            fontVariantNumeric: 'tabular-nums',
          }}
        >
          {format(value)}
        </span>
        <ComparisonArrow trend={trend} changePct={changePct} />
      </div>

      {/* 内嵌 Sparkline */}
      <Sparkline data={sparklineData} height={48} />
    </Card>
  );
}

/** 默认数字格式化：中文千分位 */
function defaultFormat(n: number): string {
  return n.toLocaleString('zh-CN');
}
