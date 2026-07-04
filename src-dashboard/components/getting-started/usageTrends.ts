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
 * 12 种高区分度颜色，使用 Tailwind CSS 色板中的 400/500 色阶，
 * 确保相邻颜色在色相环上均匀分布，同时兼顾面积图半透明填充时的视觉清晰度。
 *
 * 色相分布：
 *   #60a5fa → 蓝（400）    #f59e0b → 琥珀（500）  #34d399 → 翠绿（400）
 *   #f87171 → 红（400）    #2dd4bf → 青（400）    #fb923c → 橙（400）
 *   #a78bfa → 紫（400）    #94a3b8 → 灰（400）    #f472b6 → 粉（400）
 *   #a3e635 → 柠绿（400）  #818cf8 → 靛蓝（400）  #fb7185 → 玫红（400）
 */
const MODEL_CHART_COLORS = [
  '#60a5fa', // 蓝 — Claude
  '#f59e0b', // 琥珀 — GPT
  '#34d399', // 翠绿 — Gemini
  '#f87171', // 红 — 异常 / 兜底
  '#2dd4bf', // 青 — Cohere
  '#fb923c', // 橙 — Mistral
  '#a78bfa', // 紫 — 通用
  '#94a3b8', // 灰 — 其他 / 未知
  '#f472b6', // 粉 — 创意模型
  '#a3e635', // 柠绿 — 轻量模型
  '#818cf8', // 靛蓝 — 编码模型
  '#fb7185', // 玫红 — 备用
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
