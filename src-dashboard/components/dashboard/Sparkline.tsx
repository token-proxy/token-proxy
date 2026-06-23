import { useEffect, useState } from 'react';
import { Line, LineChart, ResponsiveContainer } from 'recharts';
import { useTheme } from '../../hooks/useTheme';

/**
 * Sparkline 组件 props。
 */
interface SparklineProps {
  /** 数据点序列（按时间顺序排列的数值） */
  data: number[];
  /** 高度，默认 48px */
  height?: number;
  /** 自定义颜色；未提供时从 Semi 主题派生品牌色 */
  color?: string;
}

/**
 * 极简单色趋势曲线，作为 KpiCard 内嵌的迷你时间序列视图。
 *
 * 设计原则（Linear / Vercel Analytics 风格）：
 * - 无坐标轴、无网格线、无图例、无 tooltip
 * - 单一品牌色 + 1.5px 描边
 * - 空数据或全零数据时显示 "—" 占位
 *
 * 主题适配：未指定 `color` 时通过 `getComputedStyle` 读取 `--semi-color-primary` CSS
 * 变量，跟随明暗主题自动切换。`useTheme()` 返回的 `effectiveTheme` 作为依赖项触发重读。
 *
 * @example
 * <Sparkline data={[1, 5, 3, 8, 6, 12, 9]} />
 */
export function Sparkline({ data, height = 48, color }: SparklineProps) {
  const { effectiveTheme } = useTheme();
  // 主题色订阅（外部状态）：仅在未显式指定 color 时启用，effect 负责跟随 effectiveTheme 重读 CSS 变量
  const [themeColor, setThemeColor] = useState<string | null>(null);

  useEffect(() => {
    // 显式指定颜色时无需订阅主题，直接跳过 effect 体
    if (color) {
      return;
    }
    // 使用 rAF 等待主题切换的 DOM 属性写入生效后再读取 CSS 变量
    const id = requestAnimationFrame(() => {
      const cssColor = getComputedStyle(document.body)
        .getPropertyValue('--semi-color-primary')
        .trim();
      if (cssColor) {
        setThemeColor(cssColor);
      }
    });
    return () => cancelAnimationFrame(id);
  }, [color, effectiveTheme]);

  // 颜色优先级：显式 color → 主题派生色 → fallback
  const resolvedColor = color ?? themeColor ?? '#3b82f6';

  // 空数据态：空数组或全零序列均视为无数据
  const isEmpty = data.length === 0 || data.every((value) => value === 0);
  if (isEmpty) {
    return (
      <div
        style={{
          height,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--semi-color-tertiary)',
          fontSize: 14,
        }}
        aria-label="无数据"
      >
        —
      </div>
    );
  }

  // Recharts 要求对象数组格式，dataKey 指向 value 字段
  const chartData = data.map((value, index) => ({ index, value }));

  return (
    <ResponsiveContainer width="100%" height={height}>
      <LineChart data={chartData} margin={{ top: 4, right: 0, bottom: 4, left: 0 }}>
        <Line
          type="monotone"
          dataKey="value"
          stroke={resolvedColor}
          strokeWidth={1.5}
          dot={false}
          isAnimationActive={false}
        />
      </LineChart>
    </ResponsiveContainer>
  );
}
