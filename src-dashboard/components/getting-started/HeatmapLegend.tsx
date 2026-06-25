/**
 * 热力图图例：5 档色阶，左下角水平排列。
 *
 * 每个色块 Hover 显示该档位的数值范围 Tooltip；
 * 不展示「少 / 多」静态文字。
 */

import { Tooltip } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import type { HeatmapLegendRanges } from './heatmapUtils';

const LEGEND_LEVELS = [0, 1, 2, 3, 4] as const;

/** HeatmapLegend 组件 Props */
interface HeatmapLegendProps {
  /** 各档位范围字符串 */
  ranges: HeatmapLegendRanges;
}

/**
 * 热力图图例组件。
 */
export function HeatmapLegend({ ranges }: HeatmapLegendProps): ReactNode {
  return (
    <div className="heatmap-legend" aria-label="活跃度图例">
      {LEGEND_LEVELS.map((level) => (
        <Tooltip key={level} content={ranges[level]} position="top">
          <div className="heatmap-legend-cell" data-level={level} />
        </Tooltip>
      ))}
    </div>
  );
}
