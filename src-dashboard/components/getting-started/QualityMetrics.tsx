/**
 * QualityMetrics - 服务调用质量指标卡片。
 *
 * 在 Dashboard 上以 3 列 × 2 行 Grid 紧凑展示 6 项指标：
 * 成功率 / 4xx 率 / 5xx 率 / 中断率 / 平均耗时 / P95 耗时。
 *
 * 比率类指标在样本数为 0 时返回 null（显示 `—`）；
 * 耗时小于 1s 显示 `xxx ms`，大于等于 1s 显示 `x.xx s`。
 */

import { Card, Skeleton } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import type { QualityResponse } from '../../types/dashboard';

/**
 * QualityMetrics 组件 Props。
 */
interface QualityMetricsProps {
  /** 服务质量响应（含总样本数与 6 项指标） */
  data: QualityResponse;
  /** 加载态：true 时渲染 Skeleton 占位 */
  loading?: boolean;
}

/**
 * 单元格指标定义。
 */
interface MetricCell {
  /** 指标展示名 */
  label: string;
  /** 已格式化的数值文本（含 `—` 占位） */
  value: string;
  /** 是否为重点指标（成功率使用更深色） */
  emphasis?: boolean;
}

/**
 * 服务质量指标卡片。
 *
 * 视觉策略：
 * - Grid 布局：`repeat(3, minmax(0, 1fr))`，3 列等宽，窄屏自适应
 * - 每格上方为指标名（13px 灰字），下方为大数字（22px 主色）
 * - 比率与耗时统一使用 tabular-nums 等宽数字，对齐美观
 * - 当 total_count 为 0 时显示整卡空态文案，不渲染网格
 *
 * @example
 * <QualityMetrics data={qualityResponse} />
 */
export function QualityMetrics({ data, loading = false }: QualityMetricsProps): ReactNode {
  // 1. 加载态
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

  // 2. 6 项指标（按 3×2 Grid 顺序排列）
  const cells = [
    { label: '成功率', value: formatRate(data.success_rate.rate), emphasis: true },
    { label: '4xx 率', value: formatRate(data.client_error_rate) },
    { label: '5xx 率', value: formatRate(data.server_error_rate) },
    { label: '中断率', value: formatRate(data.interrupted_rate) },
    { label: '平均耗时', value: formatDurationMs(data.avg_duration_ms) },
    { label: 'P95 耗时', value: formatDurationMs(data.p95_duration_ms) },
  ] satisfies MetricCell[];

  return (
    <Card
      bordered={false}
      style={{ backgroundColor: 'var(--semi-color-bg-2)', borderRadius: 12 }}
      bodyStyle={{ padding: 20 }}
    >
      {/* 标题 */}
      <div
        style={{
          fontSize: 14,
          fontWeight: 500,
          letterSpacing: '0.02em',
          color: 'var(--semi-color-text-2)',
          marginBottom: 14,
        }}
      >
        调用质量
      </div>

      {/* 空态 */}
      {data.total_count === 0 ? (
        <div
          style={{
            padding: '24px 0',
            textAlign: 'center',
            color: 'var(--semi-color-text-2)',
            fontSize: 13,
          }}
        >
          时间窗口内无调用数据
        </div>
      ) : (
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
            rowGap: 18,
            columnGap: 16,
          }}
        >
          {cells.map((cell) => (
            <div key={cell.label} style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              <span style={{ fontSize: 12, color: 'var(--semi-color-text-2)' }}>{cell.label}</span>
              <span
                style={{
                  fontSize: 22,
                  fontWeight: 600,
                  lineHeight: 1.1,
                  color: cell.emphasis ? 'var(--semi-color-success)' : 'var(--semi-color-text-0)',
                  fontVariantNumeric: 'tabular-nums',
                }}
              >
                {cell.value}
              </span>
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}

/**
 * 格式化比率（0.0 - 1.0）为百分比字符串。
 *
 * - `null` → `—`（无样本或无法计算）
 * - 否则乘以 100 保留 1 位小数 + `%`
 */
function formatRate(rate: number | null): string {
  if (rate == null) return '—';
  return `${(rate * 100).toFixed(1)}%`;
}

/**
 * 格式化耗时（毫秒）。
 *
 * - `null` → `—`
 * - `< 1000ms` → 整数毫秒（如 `342 ms`）
 * - `>= 1000ms` → 秒，保留 2 位小数（如 `1.23 s`）
 */
function formatDurationMs(ms: number | null): string {
  if (ms == null) return '—';
  if (ms < 1000) return `${Math.round(ms)} ms`;
  return `${(ms / 1000).toFixed(2)} s`;
}
