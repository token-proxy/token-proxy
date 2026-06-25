/**
 * TotalTokensCard - 词元总量卡片（KpiCard + 5 维度堆叠条 + 缓存命中率胶囊）。
 *
 * 上半部分复用 KpiCard 渲染主数值 + Sparkline + 同比箭头；
 * 下半部分以 5 段堆叠条展示词元构成，并在标题行附加缓存命中率胶囊。
 *
 * 视觉上与请求数 KPI 卡等高对齐，堆叠条仅占少量额外高度。
 */

import type { ReactNode } from 'react';
import type { CacheHitRate, KpiTrendItem, TokenComposition } from '../../types/dashboard';
import { formatTokenCompact } from '../../utils/format';
import { ComparisonArrow } from './ComparisonArrow';
import { KpiCard } from './KpiCard';
import { StackedBar, type StackedBarSegment } from './StackedBar';
import './TotalTokensCard.css';

/** 5 段词元构成的颜色与标签定义（与 TokenComposition 共享） */
const COMPOSITION_SEGMENTS = [
  { key: 'input_tokens', label: '输入', color: '#3b82f6' },
  { key: 'output_tokens', label: '输出', color: '#10b981' },
  { key: 'cache_creation_tokens', label: '缓存创建', color: '#f59e0b' },
  { key: 'cache_read_tokens', label: '缓存读取', color: '#06b6d4' },
  { key: 'thinking_tokens', label: '思考', color: '#a855f7' },
] as const;

/** TotalTokensCard 组件 Props */
interface TotalTokensCardProps {
  /** 卡片标题（如 "我的词元总量"） */
  title: string;
  /** 词元总量 KPI（含 current / change_pct / trend）；undefined 视为加载中 */
  kpi: KpiTrendItem | undefined;
  /** 词元构成 5 维度；undefined 时仅渲染主 KPI，不显示分解行 */
  composition: TokenComposition | undefined;
  /** 缓存命中率（用于标题行胶囊展示） */
  cacheHitRate?: CacheHitRate;
  /** Sparkline 数据点序列 */
  sparklineData: number[];
  /** 加载态：true 时 KpiCard 渲染 Skeleton，分解行隐藏 */
  loading?: boolean;
}

/**
 * 词元总量卡片。
 *
 * 上半部分等同于一张标准 KpiCard，下半部分以 11px 灰字横向列出 5 项分解，
 * 窄屏自动换行。所有数值均通过 `formatTokenCompact` 压缩为 K / M 单位。
 *
 * 加载中或 `composition` 缺失时仅渲染上半 KpiCard，分解行不占位，
 * 整体高度由 KpiCard 决定，加上分解行后略高于其他 KPI 卡。
 *
 * @example
 * <TotalTokensCard
 *   title="我的词元总量"
 *   kpi={kpiResponse.total_tokens}
 *   composition={kpiResponse.composition}
 *   sparklineData={[100, 200, 150, 300]}
 * />
 */
export function TotalTokensCard({
  title,
  kpi,
  composition,
  cacheHitRate,
  sparklineData,
  loading = false,
}: TotalTokensCardProps): ReactNode {
  const showBreakdown = !loading && composition !== undefined;

  // 计算总量，供堆叠条与缓存命中率使用
  const total =
    composition !== undefined
      ? composition.input_tokens +
        composition.output_tokens +
        composition.cache_creation_tokens +
        composition.cache_read_tokens +
        composition.thinking_tokens
      : 0;

  // 缓存命中率展示文本
  const cacheRateText =
    cacheHitRate?.rate == null ? '—' : `${(cacheHitRate.rate * 100).toFixed(1)}%`;

  return (
    <div className="total-tokens-card">
      <KpiCard
        title={title}
        value={kpi?.current ?? 0}
        format={formatTokenCompact}
        trend={kpi?.trend ?? 'empty'}
        changePct={kpi?.change_pct ?? null}
        sparklineData={sparklineData}
        loading={loading}
      />
      {showBreakdown && total > 0 && (
        <div className="total-tokens-breakdown">
          {/* 缓存命中率胶囊 */}
          {cacheHitRate && (
            <div className="total-tokens-cache-pill">
              <span className="total-tokens-cache-label">缓存命中率</span>
              <span className="total-tokens-cache-value">{cacheRateText}</span>
              <ComparisonArrow
                trend={cacheHitRate.trend}
                changePct={cacheHitRate.change_pct}
                showText={false}
              />
            </div>
          )}

          {/* 5 段堆叠条 */}
          <StackedBar
            segments={COMPOSITION_SEGMENTS.map(
              (s) =>
                ({
                  label: s.label,
                  value: composition[s.key],
                  color: s.color,
                }) satisfies StackedBarSegment,
            )}
            height={10}
          />

          {/* 图例：5 项横向排列 */}
          <div className="total-tokens-composition-legend">
            {COMPOSITION_SEGMENTS.map((seg) => {
              const val = composition[seg.key];
              const pct = total > 0 ? ((val / total) * 100).toFixed(1) : '0.0';
              return (
                <span key={seg.key} className="total-tokens-legend-item">
                  <span
                    className="total-tokens-legend-dot"
                    style={{ backgroundColor: seg.color }}
                  />
                  <span>{seg.label}</span>
                  <span className="total-tokens-legend-value">{formatTokenCompact(val)}</span>
                  <span className="total-tokens-legend-pct">{pct}%</span>
                </span>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
