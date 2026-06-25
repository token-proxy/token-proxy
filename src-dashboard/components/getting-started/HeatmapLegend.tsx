/**
 * 热力图图例：5 档色阶，左下角水平排列。
 *
 * 使用与 Heatmap 统一的自定义 CSS tooltip（position: fixed + 单例 state），
 * 悬浮时在色块上方居中显示该档位的数值范围。
 */

import { useCallback, useState, type ReactNode } from 'react';
import type { HeatmapLegendRanges } from './heatmapUtils';

const LEGEND_LEVELS = [0, 1, 2, 3, 4] as const;

/** Tooltip 可见状态：定位以色块上方居中为基准 */
interface TooltipState {
  /** 提示文本 */
  text: string;
  /** 色块水平中心（viewport 坐标） */
  cx: number;
  /** 色块顶边（viewport 坐标） */
  top: number;
}

/** HeatmapLegend 组件 Props */
interface HeatmapLegendProps {
  /** 各档位范围字符串 */
  ranges: HeatmapLegendRanges;
}

/**
 * 热力图图例组件。
 *
 * 使用单例 tooltip 模式替代 Semi `<Tooltip>`，
 * 与热力图格子 tooltip 共享 `.heatmap-tooltip` CSS 类，
 * 确保视觉一致且减少组件库依赖。
 */
export function HeatmapLegend({ ranges }: HeatmapLegendProps): ReactNode {
  const [tooltip, setTooltip] = useState<TooltipState | null>(null);

  const handleEnter = useCallback((e: React.MouseEvent, text: string) => {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    setTooltip({ text, cx: rect.left + rect.width / 2, top: rect.top });
  }, []);

  const handleLeave = useCallback(() => setTooltip(null), []);

  return (
    <div className="heatmap-legend" aria-label="活跃度图例">
      {LEGEND_LEVELS.map((level) => (
        <div
          key={level}
          className="heatmap-legend-cell"
          data-level={level}
          onMouseEnter={(e) => handleEnter(e, ranges[level])}
          onMouseMove={(e) => handleEnter(e, ranges[level])}
          onMouseLeave={handleLeave}
        />
      ))}
      {tooltip && (
        <div className="heatmap-tooltip" style={{ left: tooltip.cx, top: tooltip.top - 8 }}>
          {tooltip.text}
        </div>
      )}
    </div>
  );
}
