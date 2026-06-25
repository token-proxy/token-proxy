/**
 * 用量趋势图表纯函数与配置。
 *
 * 组件层只负责状态与渲染，本文件集中维护词元维度、颜色、日期格式化和空态判断，
 * 便于后续扩展新词元类型或调整视觉表达。
 */

import type { UsageTrendBucket } from '../../types/dashboard';

/** 用量趋势图支持的词元类型字段。 */
export type UsageTrendTokenKey =
  | 'input_tokens'
  | 'output_tokens'
  | 'cache_creation_tokens'
  | 'cache_read_tokens'
  | 'thinking_tokens';

export type UsageTrendTokenGroup = 'input' | 'output';

/** 词元类型展示配置。 */
export interface UsageTrendTokenConfig {
  /** 数据字段名 */
  key: UsageTrendTokenKey;
  /** 中文图例名 */
  label: string;
  /** tooltip 分组 */
  group: UsageTrendTokenGroup;
  /** 非黑色图表颜色 */
  color: string;
}

/** 非黑色固定色板，避免 CSS 变量不可用时图表退回黑色。 */
const TOKEN_COLORS = {
  input: '#2563eb',
  output: '#16a34a',
  cacheCreation: '#f97316',
  cacheRead: '#8b5cf6',
  thinking: '#ec4899',
} as const;

/** 词元维度配置，顺序即堆叠顺序与图例顺序。 */
export const TOKEN_CONFIGS = [
  { key: 'input_tokens', label: '缓存未命中', group: 'input', color: TOKEN_COLORS.input },
  {
    key: 'cache_creation_tokens',
    label: '缓存创建',
    group: 'input',
    color: TOKEN_COLORS.cacheCreation,
  },
  { key: 'cache_read_tokens', label: '缓存命中', group: 'input', color: TOKEN_COLORS.cacheRead },
  { key: 'output_tokens', label: '输出', group: 'output', color: TOKEN_COLORS.output },
  { key: 'thinking_tokens', label: '思考', group: 'output', color: TOKEN_COLORS.thinking },
] satisfies UsageTrendTokenConfig[];

// ─── 模型消费图颜色 ───────────────────────────────────────

/**
 * 模型消费面积图色板。
 *
 * 12 种视觉区分度高的 hex 颜色，用于为不同模型名分配不同的面积曲线颜色。
 * 与词元堆叠柱状图的颜色区分开，避免视觉混淆。
 */
const MODEL_CHART_COLORS = [
  '#6366f1', // indigo
  '#f59e0b', // amber
  '#10b981', // emerald
  '#ef4444', // red
  '#06b6d4', // cyan
  '#f97316', // orange
  '#8b5cf6', // violet
  '#14b8a6', // teal
  '#ec4899', // pink
  '#84cc16', // lime
  '#3b82f6', // blue
  '#e11d48', // rose
] as const;

/**
 * 基于 DJB2 算法将模型名映射为稳定颜色。
 *
 * 与 `AutoColoredTag` 的 `hashToColor` 使用相同算法，
 * 区别在于返回 hex 色值而非 Semi Design TagColor 名，适合图表渲染。
 *
 * @param model - 模型名
 * @returns 色板中的一个 hex 颜色值
 */
export function hashModelToColor(model: string): string {
  let hash = 5381;
  for (let i = 0; i < model.length; i++) {
    hash = ((hash << 5) + hash + model.charCodeAt(i)) | 0;
  }
  const index = Math.abs(hash) % MODEL_CHART_COLORS.length;
  return MODEL_CHART_COLORS[index];
}

/** 格式化普通数值，超过万级时使用紧凑表示。 */
export function formatTrendNumber(value: number | null | undefined): string {
  if (value == null) return '—';
  return new Intl.NumberFormat('zh-CN', {
    notation: value >= 10000 ? 'compact' : 'standard',
    maximumFractionDigits: value >= 10000 ? 1 : 0,
  }).format(value);
}

/** 格式化趋势桶日期标签。 */
export function formatTrendDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat('zh-CN', {
    month: 'numeric',
    day: 'numeric',
  }).format(date);
}

/** 格式化 tooltip 中的完整日期。 */
export function formatTrendTooltipDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).format(date);
}

/** 计算单个桶的词元总量。 */
export function totalTokensOfBucket(bucket: UsageTrendBucket): number {
  return TOKEN_CONFIGS.reduce((sum, item) => sum + bucket[item.key], 0);
}

/** 判断趋势数据是否完全为空。 */
export function isUsageTrendsEmpty(buckets: UsageTrendBucket[] | null | undefined): boolean {
  if (!buckets || buckets.length === 0) return true;
  return buckets.every(
    (bucket) =>
      bucket.request_count === 0 && bucket.session_count === 0 && totalTokensOfBucket(bucket) === 0,
  );
}
