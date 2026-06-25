/**
 * 用量热力图（GitHub 风格 53 × 7 方格矩阵）。
 *
 * 设计要点：
 *
 * 1. 单格 11px × 11px，53 列 × 7 行，本周固定在最右列
 * 2. 固定阈值分级（无色阶被重度用户压制的问题）
 * 3. 请求次数（蓝色系）与词元总量（绿色系）独立色板
 * 4. Hover 显示 Tooltip（格子上方居中 + 底边三角指引）
 * 5. 月份标签位于该月首次出现的列、周几仅显示一 / 三 / 五
 */

import { Button, ButtonGroup, Typography } from '@douyinfe/semi-ui';
import { useCallback, useMemo, useState, type ReactNode } from 'react';
import type { HeatmapCell } from '../../types/dashboard';
import { formatNumber, formatTokenCompact } from '../../utils/format';
import {
  cellValue,
  classify,
  getFixedLegendRanges,
  layoutCells,
  type HeatmapMetric,
} from './heatmapUtils';
import { HeatmapLegend } from './HeatmapLegend';
import './heatmap.css';

/** Heatmap 组件 props */
interface HeatmapProps {
  cells: HeatmapCell[];
  loading?: boolean;
}

/** Tooltip 可见状态：定位以格子上方居中为基准 */
interface TooltipState {
  text: string;
  /** 单元格水平中心（viewport 坐标） */
  cx: number;
  /** 单元格顶边（viewport 坐标） */
  top: number;
}

/**
 * 用量热力图主组件。
 */
export function Heatmap({ cells, loading = false }: HeatmapProps): ReactNode {
  const [metric, setMetric] = useState<HeatmapMetric>('tokens');
  const [tooltip, setTooltip] = useState<TooltipState | null>(null);

  const legendRanges = useMemo(() => getFixedLegendRanges(metric), [metric]);
  const layout = useMemo(() => layoutCells(cells), [cells]);

  // 推导月份标签：{ col, text }，仅在该月首次出现的列放置
  const monthLabels = useMemo(() => {
    const sorted = [...layout].sort((a, b) => a.col - b.col || a.row - b.row);
    const labels: { col: number; text: string }[] = [];
    const seen = new Set<string>();
    let lastCol = -1;
    for (const item of sorted) {
      if (item.col === lastCol) continue;
      lastCol = item.col;
      const monthKey = item.cell.day.slice(0, 7);
      if (seen.has(monthKey)) continue;
      seen.add(monthKey);
      const m = Number(item.cell.day.slice(5, 7));
      if (!Number.isFinite(m)) continue;
      labels.push({ col: item.col, text: String(m) });
    }
    return labels;
  }, [layout]);

  const handleCellEnter = useCallback(
    (e: React.MouseEvent, cell: HeatmapCell) => {
      const value = cellValue(cell, metric);
      const text =
        value === 0
          ? `${cell.day} · 无请求`
          : `${cell.day} · ${formatNumber(cell.request_count)} 次请求 · ${formatTokenCompact(cell.total_tokens)} 词元`;
      // 以单元格 bounding rect 定位：上方居中
      const rect = (e.target as HTMLElement).getBoundingClientRect();
      setTooltip({ text, cx: rect.left + rect.width / 2, top: rect.top });
    },
    [metric],
  );

  const handleCellLeave = useCallback(() => setTooltip(null), []);
  const handleCellMove = useCallback(
    (e: React.MouseEvent) => {
      if (!tooltip) return;
      const rect = (e.target as HTMLElement).getBoundingClientRect();
      setTooltip({ ...tooltip, cx: rect.left + rect.width / 2, top: rect.top });
    },
    [tooltip],
  );

  if (loading) {
    return <div className="heatmap-container heatmap-skeleton">加载中…</div>;
  }

  return (
    <div className="heatmap-container" data-metric={metric}>
      {/* 标题 + 单位切换 */}
      <div className="heatmap-header-row">
        <Typography.Title heading={6} className="heatmap-title">
          近 1 年用量
        </Typography.Title>
        <ButtonGroup size="small" className="heatmap-metric-toggle" aria-label="热力图单位">
          <Button
            theme={metric === 'requests' ? 'solid' : 'light'}
            type="primary"
            onClick={() => setMetric('requests')}
            aria-pressed={metric === 'requests'}
          >
            请求次数
          </Button>
          <Button
            theme={metric === 'tokens' ? 'solid' : 'light'}
            type="primary"
            onClick={() => setMetric('tokens')}
            aria-pressed={metric === 'tokens'}
          >
            词元总量
          </Button>
        </ButtonGroup>
      </div>

      {/* 统一 Grid：54 列 × 8 行（1 周几列 + 53 数据列） ×（1 月行 + 7 数据行） */}
      <div className="heatmap-grid" role="grid" aria-label="近 1 年每日用量">
        {/* 第 1 行：月份标头（该月首次出现的列，列内居中） */}
        {monthLabels.map((label) => (
          <span
            key={`m-${label.col}`}
            className="heatmap-month-label"
            style={{ gridColumn: label.col + 2, gridRow: 1 }}
          >
            {label.text}
          </span>
        ))}

        {/* 第 1 列：周几标签（仅周一/三/五，从第 2 行开始） */}
        {['一', '', '三', '', '五', '', ''].map((text, i) => (
          <span
            key={`w-${i}`}
            className="heatmap-weekday-label"
            style={{ gridColumn: 1, gridRow: i + 2 }}
          >
            {text}
          </span>
        ))}

        {/* 格子区域：col 2-54, row 2-8 */}
        {layout.map(({ col, row, cell }) => {
          const value = cellValue(cell, metric);
          const level = classify(value, metric);
          return (
            <div
              key={cell.day}
              className="heatmap-cell"
              data-level={level}
              role="gridcell"
              style={{ gridColumn: col + 2, gridRow: row + 2 }}
              onMouseEnter={(e) => handleCellEnter(e, cell)}
              onMouseMove={handleCellMove}
              onMouseLeave={handleCellLeave}
            />
          );
        })}
      </div>

      <HeatmapLegend ranges={legendRanges} />

      {/* Tooltip：格子上方居中 + 底边三角指引 */}
      {tooltip && (
        <div className="heatmap-tooltip" style={{ left: tooltip.cx, top: tooltip.top - 8 }}>
          {tooltip.text}
        </div>
      )}
    </div>
  );
}
