/**
 * TokenComposition - 词元构成卡片。
 *
 * 用于 Dashboard 展示当前时间窗口内 5 维度词元（输入 / 输出 / 缓存创建 / 缓存读取 / 思考）
 * 的占比与绝对值，并在右上角附带缓存命中率胶囊（含对比箭头）。
 *
 * 视觉策略：
 * - 上方一条 100% 堆叠条按比例切分 5 段，使整体占比一目了然
 * - 下方图例以「色块 · 维度名 · 绝对值 · 百分比」呈现具体数据
 * - 5 段颜色固定，与图例色块保持一一对应
 * - 总和为 0 时直接渲染空态文本，避免显示空条
 */

import { Card, Skeleton, Tag } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import type { CacheHitRate, TokenComposition as TokenCompositionData } from '../../types/dashboard';
import { formatTokenCompact } from '../../utils/format';
import { ComparisonArrow } from './ComparisonArrow';
import { StackedBar, type StackedBarSegment } from './StackedBar';

/**
 * TokenComposition 组件 Props。
 */
interface TokenCompositionProps {
  /** 5 维度词元绝对值 */
  composition: TokenCompositionData;
  /** 缓存命中率（含趋势对比，用于胶囊展示） */
  cacheHitRate: CacheHitRate;
  /** 加载态：true 时整体渲染 Skeleton 占位 */
  loading?: boolean;
}

/**
 * 单段图例的颜色与文案配置。
 *
 * 顺序即堆叠条从左到右的渲染顺序；颜色取自 Tailwind 风格的语义色板，
 * 与项目主色（Semi primary）形成对比但不冲突。
 */
interface SegmentSpec {
  /** 维度键（用于从 composition 提取数值） */
  key: keyof TokenCompositionData;
  /** 维度展示名（中文） */
  label: string;
  /** 段色（hex 固定值，确保跨主题视觉一致） */
  color: string;
}

const SEGMENT_SPECS = [
  { key: 'input_tokens', label: '输入', color: '#3b82f6' },
  { key: 'output_tokens', label: '输出', color: '#10b981' },
  { key: 'cache_creation_tokens', label: '缓存创建', color: '#f59e0b' },
  { key: 'cache_read_tokens', label: '缓存读取', color: '#06b6d4' },
  { key: 'thinking_tokens', label: '思考', color: '#a855f7' },
] as const satisfies readonly SegmentSpec[];

/**
 * 词元构成卡片。
 *
 * 布局自上而下：
 * 1. 标题 + 右侧缓存命中率胶囊
 * 2. 100% 堆叠条（高度 12px）
 * 3. 图例栅格：5 项横向排列，窄屏自动换行
 *
 * 边界处理：
 * - 总和为 0 → 渲染空态文案（不显示堆叠条与图例）
 * - 命中率 rate 为 null → 胶囊展示 `—`
 * - loading=true → 整卡 Skeleton
 *
 * @example
 * <TokenComposition
 *   composition={kpi.composition}
 *   cacheHitRate={kpi.cache_hit_rate}
 * />
 */
export function TokenComposition({
  composition,
  cacheHitRate,
  loading = false,
}: TokenCompositionProps): ReactNode {
  // 1. 加载态：整卡 Skeleton 占位
  if (loading) {
    return (
      <Card
        bordered={false}
        style={{ backgroundColor: 'var(--semi-color-bg-2)', borderRadius: 12 }}
        bodyStyle={{ padding: 20 }}
      >
        <Skeleton active placeholder={<Skeleton.Paragraph rows={4} />} loading={true} />
      </Card>
    );
  }

  // 2. 计算总量与各段数据
  const total =
    composition.input_tokens +
    composition.output_tokens +
    composition.cache_creation_tokens +
    composition.cache_read_tokens +
    composition.thinking_tokens;

  const segments = SEGMENT_SPECS.map((spec) => ({
    key: spec.key,
    label: spec.label,
    color: spec.color,
    value: composition[spec.key],
  }));

  const cacheRateText =
    cacheHitRate.rate == null ? '—' : `${(cacheHitRate.rate * 100).toFixed(1)}%`;

  return (
    <Card
      bordered={false}
      style={{ backgroundColor: 'var(--semi-color-bg-2)', borderRadius: 12 }}
      bodyStyle={{ padding: 20 }}
    >
      {/* 标题行：左侧标题 + 右侧命中率胶囊 */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          marginBottom: 14,
          gap: 12,
        }}
      >
        <span
          style={{
            fontSize: 13,
            color: 'var(--semi-color-text-2)',
          }}
        >
          词元构成
        </span>
        <Tag
          color="cyan"
          shape="circle"
          size="small"
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: 6,
            padding: '0 10px',
          }}
        >
          <span style={{ fontSize: 12 }}>缓存命中率 {cacheRateText}</span>
          <ComparisonArrow
            trend={cacheHitRate.trend}
            changePct={cacheHitRate.change_pct}
            showText={false}
          />
        </Tag>
      </div>

      {/* 空态：总和为 0 */}
      {total === 0 ? (
        <div
          style={{
            padding: '24px 0',
            textAlign: 'center',
            color: 'var(--semi-color-text-2)',
            fontSize: 13,
          }}
        >
          时间窗口内暂无词元数据
        </div>
      ) : (
        <>
          {/* 100% 堆叠条 */}
          <StackedBar
            segments={segments.map(
              (s) =>
                ({
                  label: s.label,
                  value: s.value,
                  color: s.color,
                }) satisfies StackedBarSegment,
            )}
            height={12}
          />

          {/* 图例：5 项横向排列 */}
          <div
            style={{
              display: 'flex',
              flexWrap: 'wrap',
              gap: '10px 18px',
              marginTop: 14,
              fontSize: 12,
              fontVariantNumeric: 'tabular-nums',
            }}
          >
            {segments.map((seg) => {
              const pct = total > 0 ? ((seg.value / total) * 100).toFixed(1) : '0.0';
              return (
                <div
                  key={seg.key}
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: 6,
                  }}
                >
                  {/* 色块 */}
                  <span
                    style={{
                      display: 'inline-block',
                      width: 10,
                      height: 10,
                      borderRadius: 2,
                      backgroundColor: seg.color,
                      flexShrink: 0,
                    }}
                  />
                  <span style={{ color: 'var(--semi-color-text-1)' }}>{seg.label}</span>
                  <span style={{ color: 'var(--semi-color-text-0)', fontWeight: 500 }}>
                    {formatTokenCompact(seg.value)}
                  </span>
                  <span style={{ color: 'var(--semi-color-text-2)' }}>{pct}%</span>
                </div>
              );
            })}
          </div>
        </>
      )}
    </Card>
  );
}
