/**
 * 用量趋势卡片。
 *
 * 左侧面积图展示请求数和会话数变化，中间面积图按模型展示消费词元变化，右侧堆叠柱状图展示 5 类词元构成变化。
 * 卡片自管理 30 天 / 自定义时间范围，外部只通过 refreshKey 触发统一刷新。
 */

import { type ReactNode, useCallback, useEffect, useMemo, useState } from 'react';
import { Card, Notification, Spin, Typography } from '@douyinfe/semi-ui';
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';
import { dashboardApi } from '../../api';
import { useFetch } from '../../hooks/useFetch';
import type { ModelTokenUsage, UsageTrendBucket } from '../../types/dashboard';
import { last30Range, type TimeRangeValue } from '../../types/dashboard';
import { formatNumber } from '../../utils/format';
import { TimeRangeSelector } from './TimeRangeSelector';
import {
  formatTrendDate,
  formatTrendNumber,
  formatTrendTooltipDate,
  hashModelToColor,
  isUsageTrendsEmpty,
  TOKEN_CONFIGS,
} from './usageTrends';
import { browserTimezone } from './heatmapUtils';
import './UsageTrendsCard.css';

/** Recharts tooltip payload 项。 */
interface ChartTooltipPayloadItem {
  /** 数据字段名 */
  dataKey?: string | number;
  /** 图例名 */
  name?: string;
  /** 当前值 */
  value?: number;
  /** 图形颜色 */
  color?: string;
}

/** Recharts tooltip 组件属性。 */
interface ChartTooltipProps {
  /** tooltip 是否可见 */
  active?: boolean;
  /** 当前数据点 payload */
  payload?: ChartTooltipPayloadItem[];
  /** 横轴标签 */
  label?: string;
}

type TooltipMode = 'requests' | 'tokens';

/** 自定义 tooltip 组件属性。 */
interface UsageTrendTooltipProps extends ChartTooltipProps {
  /** tooltip 展示模式 */
  mode: TooltipMode;
}

/** 图例展示项。 */
interface TrendLegendItem {
  /** 数据字段名 */
  key: string;
  /** 中文标签 */
  label: string;
  /** 当前时间范围内总量 */
  total: number;
  /** 图例颜色 */
  color: string;
  /** 数值格式化函数，默认使用趋势图紧凑量纲 */
  formatValue?: (value: number) => string;
}

/** 渲染带总量的图表图例。 */
function TrendLegend({ items }: { items: TrendLegendItem[] }): ReactNode {
  return (
    <div className="usage-trends-legend">
      {items.map((item) => (
        <span className="usage-trends-legend-item" key={item.key}>
          <span className="usage-trends-legend-dot" style={{ background: item.color }} />
          <span>{item.label}</span>
          <strong>{(item.formatValue ?? formatTrendNumber)(item.total)}</strong>
        </span>
      ))}
    </div>
  );
}

function payloadValue(payload: ChartTooltipPayloadItem[], key: string): number {
  return payload.find((item) => item.dataKey === key)?.value ?? 0;
}

function renderTooltipRow(item: ChartTooltipPayloadItem): ReactNode {
  return (
    <div className="usage-trends-tooltip-row" key={String(item.dataKey)}>
      <span className="usage-trends-tooltip-dot" style={{ background: item.color }} />
      <span className="usage-trends-tooltip-label">{item.name}</span>
      <span className="usage-trends-tooltip-value">{formatNumber(item.value ?? 0)}</span>
    </div>
  );
}

function renderTooltipSummary(label: string, value: number): ReactNode {
  return (
    <div className="usage-trends-tooltip-summary">
      <span>{label}</span>
      <strong>{formatNumber(value)}</strong>
    </div>
  );
}

/** 自定义 tooltip，统一请求数面积图与词元柱状图样式。 */
function UsageTrendTooltip({ active, payload, label, mode }: UsageTrendTooltipProps): ReactNode {
  if (!active || !payload?.length || !label) return null;

  if (mode === 'tokens') {
    const inputTotal =
      payloadValue(payload, 'input_tokens') +
      payloadValue(payload, 'cache_creation_tokens') +
      payloadValue(payload, 'cache_read_tokens');
    const outputTotal =
      payloadValue(payload, 'output_tokens') + payloadValue(payload, 'thinking_tokens');
    const tokenTotal = inputTotal + outputTotal;
    const orderedPayload = TOKEN_CONFIGS.map((config) =>
      payload.find((item) => item.dataKey === config.key),
    ).filter((item): item is ChartTooltipPayloadItem => Boolean(item));
    const inputRows = orderedPayload.filter((item) =>
      ['input_tokens', 'cache_creation_tokens', 'cache_read_tokens'].includes(String(item.dataKey)),
    );
    const outputRows = orderedPayload.filter((item) =>
      ['output_tokens', 'thinking_tokens'].includes(String(item.dataKey)),
    );

    return (
      <div className="usage-trends-tooltip">
        <div className="usage-trends-tooltip-title">{formatTrendTooltipDate(label)}</div>
        {renderTooltipSummary('词元总量', tokenTotal)}
        <div className="usage-trends-tooltip-group">
          {renderTooltipSummary('输入词元', inputTotal)}
          {inputRows.map(renderTooltipRow)}
        </div>
        <div className="usage-trends-tooltip-group">
          {renderTooltipSummary('输出词元', outputTotal)}
          {outputRows.map(renderTooltipRow)}
        </div>
      </div>
    );
  }

  return (
    <div className="usage-trends-tooltip">
      <div className="usage-trends-tooltip-title">{formatTrendTooltipDate(label)}</div>
      {payload.map(renderTooltipRow)}
    </div>
  );
}

/** 用量趋势卡片组件。 */
export function UsageTrendsCard(): ReactNode {
  const [timeRange, setTimeRange] = useState<TimeRangeValue>(last30Range);
  const tz = useMemo(() => browserTimezone(), []);

  const fetchTrends = useCallback(
    () => dashboardApi.getUsageTrends(timeRange, tz),
    [timeRange, tz],
  );

  const { data, loading, error, refetch } = useFetch(fetchTrends, [fetchTrends]);

  useEffect(() => {
    if (error) {
      Notification.error({ title: '用量趋势加载失败', content: error, duration: 5 });
    }
  }, [error]);

  const buckets = data?.buckets ?? [];
  const empty = useMemo(() => isUsageTrendsEmpty(buckets), [buckets]);

  const chartData = useMemo(
    () =>
      buckets.map((bucket: UsageTrendBucket) => ({
        ...bucket,
        date_label: formatTrendDate(bucket.bucket_start),
      })),
    [buckets],
  );

  const requestLegendItems = useMemo<TrendLegendItem[]>(
    () => [
      {
        key: 'request_count',
        label: '请求数',
        total: buckets.reduce((sum, bucket) => sum + bucket.request_count, 0),
        color: 'var(--semi-color-primary)',
      },
      {
        key: 'session_count',
        label: '会话数',
        total: data?.window_session_count ?? 0,
        color: '#14b8a6',
      },
    ],
    [buckets, data?.window_session_count],
  );

  const tokenLegendItems = useMemo<TrendLegendItem[]>(
    () =>
      TOKEN_CONFIGS.map((item) => ({
        key: item.key,
        label: item.label,
        total: buckets.reduce((sum, bucket) => sum + bucket[item.key], 0),
        color: item.color,
        formatValue: formatNumber,
      })),
    [buckets],
  );

  // ─── 模型消费图数据 ───────────────────────────────────

  /**
   * 收集所有出现过的模型名，按总词元降序排列。
   *
   * 仅保留总词元 > 0 的模型，过滤掉窗口内全量为 0 的模型。
   */
  const modelNames = useMemo(() => {
    const totals = new Map<string, number>();
    buckets.forEach((b) => {
      b.per_model?.forEach((m: ModelTokenUsage) => {
        totals.set(m.model, (totals.get(m.model) ?? 0) + m.total_tokens);
      });
    });
    return [...totals.entries()]
      .filter(([, total]) => total > 0)
      .sort((a, b) => b[1] - a[1])
      .map(([model]) => model);
  }, [buckets]);

  /**
   * 模型消费图数据：在每个 bucket 上挂载各模型的总词元字段。
   *
   * 将 `per_model` 数组 pivot 为扁平的 `{ [modelName]: total_tokens }` key，
   * 使 Recharts 的 `<Area dataKey="modelName" />` 能直接消费。
   * 每个模型在所有 bucket 中预初始化为 0，确保该模型无数据的日期也能画出连线（回落 0）。
   */
  const modelChartData = useMemo(
    () =>
      chartData.map((bucket) => {
        const entry: Record<string, unknown> = { ...bucket };
        // 预初始化所有模型为 0，保证无数据日期线条不断开
        modelNames.forEach((model) => {
          entry[model] = 0;
        });
        if (bucket.per_model) {
          bucket.per_model.forEach((m: ModelTokenUsage) => {
            entry[m.model] = m.total_tokens;
          });
        }
        return entry;
      }),
    [chartData, modelNames],
  );

  /** 模型消费图图例项。 */
  const modelLegendItems = useMemo<TrendLegendItem[]>(
    () =>
      modelNames.map((model) => ({
        key: model,
        label: model,
        total: buckets.reduce(
          (sum, b) =>
            sum + (b.per_model?.find((m: ModelTokenUsage) => m.model === model)?.total_tokens ?? 0),
          0,
        ),
        color: hashModelToColor(model),
        formatValue: formatNumber,
      })),
    [buckets, modelNames],
  );

  const headerExtra = (
    <TimeRangeSelector
      value={timeRange}
      onChange={setTimeRange}
      onRefresh={refetch}
      loading={loading}
      allowedPresets={['last30', 'custom']}
    />
  );

  return (
    <Card className="usage-trends-card" title="用量趋势" headerExtraContent={headerExtra}>
      {loading && !data ? (
        <div className="usage-trends-state">
          <Spin size="large" />
        </div>
      ) : error ? (
        <div className="usage-trends-state usage-trends-error">{error}</div>
      ) : empty ? (
        <div className="usage-trends-state">
          <Typography.Text type="tertiary">当前时间范围暂无用量数据</Typography.Text>
        </div>
      ) : (
        <div className="usage-trends-grid">
          <section className="usage-trends-panel" aria-label="请求数趋势">
            <div className="usage-trends-panel-title">请求数 / 会话数</div>
            <TrendLegend items={requestLegendItems} />
            <ResponsiveContainer width="100%" height={260}>
              <AreaChart data={chartData} margin={{ top: 12, right: 16, bottom: 0, left: 0 }}>
                <defs>
                  <linearGradient id="usage-trends-requests" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="var(--semi-color-primary)" stopOpacity={0.32} />
                    <stop offset="95%" stopColor="var(--semi-color-primary)" stopOpacity={0.04} />
                  </linearGradient>
                  <linearGradient id="usage-trends-sessions" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#14b8a6" stopOpacity={0.28} />
                    <stop offset="95%" stopColor="#14b8a6" stopOpacity={0.04} />
                  </linearGradient>
                </defs>
                <CartesianGrid
                  stroke="var(--semi-color-border)"
                  strokeDasharray="3 3"
                  vertical={false}
                />
                <XAxis
                  dataKey="bucket_start"
                  tickFormatter={formatTrendDate}
                  tick={{ fill: 'var(--semi-color-text-2)', fontSize: 12 }}
                  axisLine={{ stroke: 'var(--semi-color-border)' }}
                  tickLine={false}
                />
                <YAxis
                  domain={[0, 'dataMax']}
                  tickFormatter={formatTrendNumber}
                  tick={{ fill: 'var(--semi-color-text-2)', fontSize: 12 }}
                  axisLine={false}
                  tickLine={false}
                  width={52}
                />
                <Tooltip content={<UsageTrendTooltip mode="requests" />} />
                <Area
                  type="monotone"
                  dataKey="request_count"
                  name="请求数"
                  stroke="var(--semi-color-primary)"
                  strokeWidth={2}
                  fill="url(#usage-trends-requests)"
                  isAnimationActive={false}
                />
                <Area
                  type="monotone"
                  dataKey="session_count"
                  name="会话数"
                  stroke="#14b8a6"
                  strokeWidth={2}
                  fill="url(#usage-trends-sessions)"
                  isAnimationActive={false}
                />
              </AreaChart>
            </ResponsiveContainer>
          </section>

          <section className="usage-trends-panel" aria-label="模型消费趋势">
            <div className="usage-trends-panel-title">模型消费</div>
            <TrendLegend items={modelLegendItems} />
            <ResponsiveContainer width="100%" height={260}>
              <AreaChart data={modelChartData} margin={{ top: 12, right: 16, bottom: 0, left: 0 }}>
                <defs>
                  {modelNames.map((model) => {
                    const color = hashModelToColor(model);
                    const id = `usage-trends-model-${model.replace(/[^a-zA-Z0-9]/g, '-')}`;
                    return (
                      <linearGradient key={model} id={id} x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor={color} stopOpacity={0.32} />
                        <stop offset="95%" stopColor={color} stopOpacity={0.04} />
                      </linearGradient>
                    );
                  })}
                </defs>
                <CartesianGrid
                  stroke="var(--semi-color-border)"
                  strokeDasharray="3 3"
                  vertical={false}
                />
                <XAxis
                  dataKey="bucket_start"
                  tickFormatter={formatTrendDate}
                  tick={{ fill: 'var(--semi-color-text-2)', fontSize: 12 }}
                  axisLine={{ stroke: 'var(--semi-color-border)' }}
                  tickLine={false}
                />
                <YAxis
                  domain={[0, 'dataMax']}
                  tickFormatter={formatTrendNumber}
                  tick={{ fill: 'var(--semi-color-text-2)', fontSize: 12 }}
                  axisLine={false}
                  tickLine={false}
                  width={52}
                />
                <Tooltip content={<UsageTrendTooltip mode="requests" />} />
                {modelNames.map((model) => {
                  const color = hashModelToColor(model);
                  const gradientId = `usage-trends-model-${model.replace(/[^a-zA-Z0-9]/g, '-')}`;
                  return (
                    <Area
                      key={model}
                      type="monotone"
                      dataKey={model}
                      name={model}
                      stroke={color}
                      strokeWidth={2}
                      fill={`url(#${gradientId})`}
                      isAnimationActive={false}
                    />
                  );
                })}
              </AreaChart>
            </ResponsiveContainer>
          </section>

          <section className="usage-trends-panel" aria-label="词元趋势">
            <div className="usage-trends-panel-title">词元构成</div>
            <TrendLegend items={tokenLegendItems} />
            <ResponsiveContainer width="100%" height={260}>
              <BarChart data={chartData} margin={{ top: 12, right: 16, bottom: 0, left: 0 }}>
                <CartesianGrid
                  stroke="var(--semi-color-border)"
                  strokeDasharray="3 3"
                  vertical={false}
                />
                <XAxis
                  dataKey="bucket_start"
                  tickFormatter={formatTrendDate}
                  tick={{ fill: 'var(--semi-color-text-2)', fontSize: 12 }}
                  axisLine={{ stroke: 'var(--semi-color-border)' }}
                  tickLine={false}
                />
                <YAxis
                  domain={[0, 'dataMax']}
                  tickFormatter={formatTrendNumber}
                  tick={{ fill: 'var(--semi-color-text-2)', fontSize: 12 }}
                  axisLine={false}
                  tickLine={false}
                  width={52}
                />
                <Tooltip content={<UsageTrendTooltip mode="tokens" />} />
                {[...TOKEN_CONFIGS].reverse().map((item) => (
                  <Bar
                    key={item.key}
                    dataKey={item.key}
                    name={item.label}
                    stackId="tokens"
                    fill={item.color}
                    radius={[3, 3, 0, 0]}
                    isAnimationActive={false}
                  />
                ))}
              </BarChart>
            </ResponsiveContainer>
          </section>
        </div>
      )}
    </Card>
  );
}
