import { Tooltip } from '@douyinfe/semi-ui';
import { formatNumber } from '../../utils/format.ts';

/**
 * 单段堆叠条数据。
 */
export interface StackedBarSegment {
  /** 段标签（用于 tooltip 和 aria-label） */
  label: string;
  /** 段数值（小于等于 0 的段不渲染） */
  value: number;
  /** 段颜色（CSS 变量或 hex） */
  color: string;
}

/**
 * StackedBar 组件 Props。
 */
interface StackedBarProps {
  /** 段数组（按显示顺序从左到右排列） */
  segments: StackedBarSegment[];
  /**
   * 总宽度的最大值，用于跨行对比时统一刻度。
   *
   * 若 segments 总和小于 maxTotal，则条形不填满，剩余空间留空（淡色背景）。
   * 不传时使用 segments 总和（条形 100% 填满）。
   */
  maxTotal?: number;
  /** 条形高度（像素），默认 8 */
  height?: number;
}

/**
 * 横向堆叠条（纯 CSS Flex 实现，不依赖图表库）。
 *
 * 用于账号排行卡片展示 input / output / cache_read / cache_creation
 * 4 段词元占比可视化。每段悬浮显示 Semi Tooltip（段名 + 数值 + 占比百分比）。
 *
 * 设计要点：
 * - 圆角通过外层 `overflow: hidden` 一次性裁剪，无需逐段处理
 * - 段宽度按 `value / total` 计算（total = maxTotal 或 segments 总和）
 * - 占比百分比按 `value / segmentsTotal` 计算（始终基于实际数据总和，避免 maxTotal 干扰）
 * - aria-label 聚合所有段标签和数值，提供无障碍读屏支持
 *
 * @example
 * <StackedBar segments={[
 *   { label: '输入', value: 1000, color: 'var(--semi-color-primary)' },
 *   { label: '输出', value: 500, color: 'var(--semi-color-success)' },
 * ]} />
 */
export function StackedBar({ segments, maxTotal, height = 8 }: StackedBarProps) {
  const segmentsTotal = segments.reduce((acc, s) => acc + s.value, 0);
  const total = maxTotal ?? segmentsTotal;

  // 总值为 0 时显示一条灰色细线作为占位
  if (total <= 0) {
    return (
      <div
        style={{
          width: '100%',
          height,
          borderRadius: height / 2,
          backgroundColor: 'var(--semi-color-fill-1)',
        }}
        aria-label="无数据"
      />
    );
  }

  return (
    <div
      style={{
        width: '100%',
        height,
        display: 'flex',
        borderRadius: height / 2,
        overflow: 'hidden',
        backgroundColor: 'var(--semi-color-fill-1)',
      }}
      role="img"
      aria-label={segments
        .filter((s) => s.value > 0)
        .map((s) => `${s.label}: ${formatNumber(s.value)}`)
        .join(', ')}
    >
      {segments.map((seg, idx) => {
        if (seg.value <= 0) return null;
        // 段宽度基于 total（可能包含 maxTotal 留白）
        const widthPct = (seg.value / total) * 100;
        // 占比百分比始终基于实际数据总和，呈现真实分布
        const pctText = ((seg.value / segmentsTotal) * 100).toFixed(1);
        return (
          <Tooltip key={idx} content={`${seg.label}: ${formatNumber(seg.value)} (${pctText}%)`}>
            <div
              style={{
                width: `${widthPct}%`,
                backgroundColor: seg.color,
                cursor: 'default',
              }}
            />
          </Tooltip>
        );
      })}
    </div>
  );
}
