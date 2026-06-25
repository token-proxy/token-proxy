/**
 * 热力图工具函数集。
 *
 * 本模块提供热力图渲染所需的纯函数：
 *
 * 1. 固定阈值分级（请求：0~10/10~100/100~1000/1000~10000/10000+，
 *                       词元：1~10w/10w~100w/100w~1000w/1000w~1亿/1亿+）
 * 2. 单元格活跃度分级（0 - 4 共 5 档）
 * 3. 365 天数据到 53 × 7 网格坐标的布局（以今日为锚点）
 * 4. 图例范围字符串生成
 * 5. 浏览器时区探测
 *
 * 模块不依赖 React 与 DOM，便于单元测试。
 */

import type { HeatmapCell } from '../../types/dashboard';

/** 热力图渲染指标 */
export type HeatmapMetric = 'requests' | 'tokens';

/** 单元格活跃度档位：`0` = 最低，`4` = 最高 */
export type HeatmapLevel = 0 | 1 | 2 | 3 | 4;

/** 单格在 53 × 7 网格中的布局结果 */
export interface HeatmapCellLayout {
  /** 列索引（0 - 52） */
  col: number;
  /** 行索引（0 = 周一，6 = 周日） */
  row: number;
  /** 原始单元格数据 */
  cell: HeatmapCell;
}

/** 固定阈值：`[低, 中低, 中高, 高]` 四个边界值 */
export type HeatmapThresholds = [number, number, number, number];

/** 图例范围五元组：`[最低档, 低档, 中档, 较高档, 最高档]` */
export type HeatmapLegendRanges = [string, string, string, string, string];

/**
 * 请求次数固定阈值。
 *
 * 档位映射：
 * - 0：≤ 10
 * - 1：11 - 100
 * - 2：101 - 1,000
 * - 3：1,001 - 10,000
 * - 4：> 10,000
 */
const REQUEST_THRESHOLDS: HeatmapThresholds = [10, 100, 1_000, 10_000];

/**
 * 词元总量固定阈值。
 *
 * 档位映射：
 * - 0：≤ 10 万
 * - 1：10 万 - 100 万
 * - 2：100 万 - 1,000 万
 * - 3：1,000 万 - 1 亿
 * - 4：> 1 亿
 */
const TOKEN_THRESHOLDS: HeatmapThresholds = [100_000, 1_000_000, 10_000_000, 100_000_000];

/** 请求次数各档位图例文字 */
const REQUEST_LEGEND: HeatmapLegendRanges = [
  '0~10',
  '11~100',
  '101~1,000',
  '1,001~10,000',
  '10,000+',
];

/** 词元总量各档位图例文字 */
const TOKEN_LEGEND: HeatmapLegendRanges = [
  '1~10 万',
  '10 万~100 万',
  '100 万~1,000 万',
  '1,000 万~1 亿',
  '1 亿+',
];

/**
 * 根据选定指标从单元格中提取数值。
 */
export function cellValue(cell: HeatmapCell, metric: HeatmapMetric): number {
  return metric === 'requests' ? cell.request_count : cell.total_tokens;
}

/**
 * 获取固定阈值。
 */
export function getFixedThresholds(metric: HeatmapMetric): HeatmapThresholds {
  return metric === 'requests' ? REQUEST_THRESHOLDS : TOKEN_THRESHOLDS;
}

/**
 * 获取固定图例范围文字。
 */
export function getFixedLegendRanges(metric: HeatmapMetric): HeatmapLegendRanges {
  return metric === 'requests' ? REQUEST_LEGEND : TOKEN_LEGEND;
}

/**
 * 根据数值与固定阈值返回 0 - 4 等级。
 *
 * - `value ≤ t₁` → `0`
 * - `t₁ < value ≤ t₂` → `1`
 * - `t₂ < value ≤ t₃` → `2`
 * - `t₃ < value ≤ t₄` → `3`
 * - `value > t₄` → `4`
 */
export function classify(value: number, metric: HeatmapMetric): HeatmapLevel {
  const [t1, t2, t3, t4] = metric === 'requests' ? REQUEST_THRESHOLDS : TOKEN_THRESHOLDS;
  if (value <= t1) return 0;
  if (value <= t2) return 1;
  if (value <= t3) return 2;
  if (value <= t4) return 3;
  return 4;
}

/**
 * 把 365 天数据布局到 53 × 7 网格的 `(col, row)` 坐标。
 *
 * GitHub 风格布局规则：
 *
 * 1. `row = ISO 周几`（周一 = 0，周日 = 6）
 * 2. `col = 周序`，当前所在周固定在最右列（col = 52）
 * 3. 早于左上角（即超出 53 周范围）的日期被丢弃
 */
export function layoutCells(cells: HeatmapCell[]): HeatmapCellLayout[] {
  const today = new Date();
  const todayUtc = Date.UTC(today.getUTCFullYear(), today.getUTCMonth(), today.getUTCDate());
  const todayMondayUtc = mondayUtcOf(todayUtc);

  const WEEK_MS = 7 * 86400_000;
  const result: HeatmapCellLayout[] = [];

  for (const cell of cells) {
    const parts = cell.day.split('-');
    if (parts.length !== 3) continue;

    const cellUtc = Date.UTC(Number(parts[0]), Number(parts[1]) - 1, Number(parts[2]));
    if (Number.isNaN(cellUtc)) continue;

    const cellMondayUtc = mondayUtcOf(cellUtc);
    const weeksAgo = Math.round((todayMondayUtc - cellMondayUtc) / WEEK_MS);
    const col = 52 - weeksAgo;

    if (col < 0 || col > 52) continue;

    const row = isoWeekdayIndex(cellUtc);
    result.push({ col, row, cell });
  }

  return result;
}

/**
 * 获取浏览器时区，找不到时回退 `'UTC'`。
 */
export function browserTimezone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone ?? 'UTC';
}

// --- 内部辅助函数 ---

function mondayUtcOf(utcMs: number): number {
  const weekday = new Date(utcMs).getUTCDay();
  return utcMs - ((weekday + 6) % 7) * 86400_000;
}

function isoWeekdayIndex(utcMs: number): number {
  return (new Date(utcMs).getUTCDay() + 6) % 7;
}
