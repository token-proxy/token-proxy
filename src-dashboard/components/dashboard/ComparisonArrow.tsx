/**
 * ComparisonArrow - 趋势对比箭头组件。
 *
 * 根据 KPI 的 TrendBadge 与百分比变化值，渲染统一的对比视觉。
 * 颜色统一通过 Semi Design CSS 变量获取，自动适配 light/dark 主题。
 */

import { IconArrowDown, IconArrowUp, IconMinus } from '@douyinfe/semi-icons';
import type { ReactNode } from 'react';
import type { TrendBadge } from '../../types/dashboard';

/** ComparisonArrow 组件 Props */
interface ComparisonArrowProps {
  /** 趋势徽章（来自后端 KpiTrendItem.trend / CacheHitRate.trend） */
  trend: TrendBadge;
  /** 百分比变化值（如 +12.3 表示上升 12.3%）；null 表示无可计算的对比 */
  changePct: number | null;
  /** 是否显示文本（默认 true）；紧凑场景可关闭只显箭头 */
  showText?: boolean;
}

/**
 * 趋势对比箭头组件。
 *
 * 根据 TrendBadge 渲染对应的视觉：
 * - `up` → 上升箭头 + 成功色 + 百分比
 * - `down` → 下降箭头 + 危险色 + 百分比
 * - `flat` → 持平短横 + 中性色 + "持平"
 * - `new` → 成功色 + "新增"（无百分比）
 * - `empty` → 中性色破折号（无对比数据）
 *
 * @example
 * <ComparisonArrow trend="up" changePct={12.5} />
 * <ComparisonArrow trend="new" changePct={null} />
 * <ComparisonArrow trend="down" changePct={-8.5} showText={false} />
 */
export function ComparisonArrow({
  trend,
  changePct,
  showText = true,
}: ComparisonArrowProps): ReactNode {
  // 1. 边界状态：无数据对比
  if (trend === 'empty') {
    return <span style={{ color: 'var(--semi-color-tertiary)', fontSize: 13 }}>—</span>;
  }

  // 2. 边界状态：上一窗为 0，本窗有值（无百分比可言）
  if (trend === 'new') {
    return (
      <span
        style={{
          color: 'var(--semi-color-success)',
          fontSize: 13,
          fontWeight: 500,
        }}
      >
        新增
      </span>
    );
  }

  // 3. 常规趋势：选择图标 / 颜色 / 文本
  let icon: ReactNode;
  let color: string;
  let text: string;

  if (trend === 'up') {
    icon = <IconArrowUp size="small" />;
    color = 'var(--semi-color-success)';
    text = formatPct(changePct);
  } else if (trend === 'down') {
    icon = <IconArrowDown size="small" />;
    color = 'var(--semi-color-danger)';
    text = formatPct(changePct);
  } else {
    // flat: 双窗都有值但变化极小（或完全相等）
    icon = <IconMinus size="small" />;
    color = 'var(--semi-color-tertiary)';
    text = '持平';
  }

  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 4,
        color,
        fontSize: 13,
        fontWeight: 500,
      }}
    >
      {icon}
      {showText && <span>{text}</span>}
    </span>
  );
}

/**
 * 格式化百分比变化值。
 *
 * - `null` → "—"（保险兜底，正常情况下 up/down 必有 changePct）
 * - 绝对值 >= 1000 → ">+1000%" / ">-1000%"（避免显示离谱大数）
 * - 否则保留 1 位小数 + 正负号
 */
function formatPct(value: number | null): string {
  if (value == null) return '—';
  if (Math.abs(value) >= 1000) {
    return value > 0 ? '>+1000%' : '>-1000%';
  }
  const sign = value > 0 ? '+' : '';
  return `${sign}${value.toFixed(1)}%`;
}
