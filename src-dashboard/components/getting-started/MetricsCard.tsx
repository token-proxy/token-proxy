/**
 * 数据指标卡片组件。
 *
 * 封装 KPI + 质量数据的获取与呈现，自包含时间范围选择器、
 * 两组指标链路、错误通知和加载骨架。
 *
 * 两项业务链路：
 * 1. 调用链路：会话数 / 请求数 / 请求成功率
 * 2. 词元链路：输入词元 / 输出词元 / 缓存命中率
 *
 * 每项指标展示当前值与环比趋势。
 */

import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { Card, Notification, Skeleton, Typography } from '@douyinfe/semi-ui';
import { dashboardApi } from '../../api';
import { TimeRangeSelector } from './TimeRangeSelector';
import { ComparisonArrow } from './ComparisonArrow';
import { useFetch } from '../../hooks/useFetch';
import { browserTimezone } from './heatmapUtils';
import type { CacheHitRate, KpiTrendItem, RateTrendItem, TrendBadge } from '../../types/dashboard';
import { last7Range, type TimeRangeValue } from '../../types/dashboard';
import { formatNumber } from '../../utils/format';

const { Text } = Typography;

/** 指标项 Props：统一适配计数型与比率型趋势 */
interface MetricCellProps {
  label: string;
  /** 格式化后的当前值字符串 */
  value: string;
  /** 趋势徽章 */
  trend: TrendBadge;
  /** 环比变化百分比；null 表示无法计算 */
  changePct: number | null;
}

/** 指标项：label → value → ComparisonArrow 纵向排列 */
function MetricCell({ label, value, trend, changePct }: MetricCellProps): ReactNode {
  return (
    <div className="metrics-cell">
      <Text type="tertiary" className="metrics-cell-label">
        {label}
      </Text>
      <span className="metrics-cell-value">{value}</span>
      <ComparisonArrow trend={trend} changePct={changePct} showText={true} />
    </div>
  );
}

/** 从 KpiTrendItem 提取 MetricCell 所需字段 */
function kpiToProps(item: KpiTrendItem): Omit<MetricCellProps, 'label' | 'value'> {
  return { trend: item.trend, changePct: item.change_pct };
}

/** 从 RateTrendItem 提取 MetricCell 所需字段 */
function rateToProps(item: RateTrendItem | CacheHitRate): Omit<MetricCellProps, 'label' | 'value'> {
  return { trend: item.trend, changePct: item.change_pct };
}

/** 格式化比率：null → '—'，否则 with 1 decimal percent */
function fmtRate(rate: number | null | undefined): string {
  if (rate == null) return '—';
  return `${(rate * 100).toFixed(1)}%`;
}

/**
 * 数据指标卡片：两组业务链路 6 项核心指标。
 */
export function MetricsCard(): ReactNode {
  const [timeRange, setTimeRange] = useState<TimeRangeValue>(last7Range);
  const tz = useMemo(() => browserTimezone(), []);

  const fetchDeps = useMemo(
    () => [timeRange.start.getTime(), timeRange.end.getTime()],
    [timeRange.start, timeRange.end],
  );

  const kpiQuery = useFetch(() => dashboardApi.getKpi(timeRange, tz), fetchDeps);
  const qualityQuery = useFetch(() => dashboardApi.getQuality(timeRange), fetchDeps);

  const isFirstLoad =
    (kpiQuery.loading || qualityQuery.loading) && !kpiQuery.data && !qualityQuery.data;
  const isRefreshing = (kpiQuery.loading || qualityQuery.loading) && !isFirstLoad;

  const handleRefresh = () => {
    kpiQuery.refetch();
    qualityQuery.refetch();
  };

  useEffect(() => {
    const firstError = [kpiQuery.error, qualityQuery.error].find(Boolean);
    if (firstError) {
      Notification.error({ title: '指标数据加载失败', content: firstError, duration: 5 });
    }
  }, [kpiQuery.error, qualityQuery.error]);

  return (
    <Card
      title="数据指标"
      className="gs-hero-right-card"
      headerExtraContent={
        <TimeRangeSelector
          value={timeRange}
          onChange={setTimeRange}
          onRefresh={handleRefresh}
          loading={isRefreshing}
        />
      }
      bodyStyle={{ padding: '16px 20px 20px' }}
    >
      {isFirstLoad ? (
        <Skeleton active placeholder={<Skeleton.Paragraph rows={6} />} loading={true} />
      ) : (
        <div className="metrics-card-content">
          {/* 调用链路：会话数 / 请求数 / 请求成功率 */}
          <div className="metrics-row">
            <MetricCell
              label="会话数"
              value={formatNumber(kpiQuery.data?.session_count.current ?? 0)}
              {...kpiToProps(
                kpiQuery.data?.session_count ?? {
                  current: 0,
                  previous: 0,
                  trend: 'empty',
                  change_pct: null,
                },
              )}
            />
            <MetricCell
              label="请求数"
              value={formatNumber(kpiQuery.data?.request_count.current ?? 0)}
              {...kpiToProps(
                kpiQuery.data?.request_count ?? {
                  current: 0,
                  previous: 0,
                  trend: 'empty',
                  change_pct: null,
                },
              )}
            />
            <MetricCell
              label="请求成功率"
              value={fmtRate(qualityQuery.data?.success_rate.rate)}
              {...rateToProps(
                qualityQuery.data?.success_rate ?? {
                  rate: null,
                  previous_rate: null,
                  change_pct: null,
                  trend: 'empty',
                },
              )}
            />
          </div>

          {/* 词元链路：输入词元 / 输出词元 / 缓存命中率 */}
          <div className="metrics-row">
            <MetricCell
              label="输入词元"
              value={formatNumber(kpiQuery.data?.input_tokens.current ?? 0)}
              {...kpiToProps(
                kpiQuery.data?.input_tokens ?? {
                  current: 0,
                  previous: 0,
                  trend: 'empty',
                  change_pct: null,
                },
              )}
            />
            <MetricCell
              label="输出词元"
              value={formatNumber(kpiQuery.data?.output_tokens.current ?? 0)}
              {...kpiToProps(
                kpiQuery.data?.output_tokens ?? {
                  current: 0,
                  previous: 0,
                  trend: 'empty',
                  change_pct: null,
                },
              )}
            />
            <MetricCell
              label="缓存命中率"
              value={fmtRate(kpiQuery.data?.cache_hit_rate.rate)}
              {...rateToProps(
                kpiQuery.data?.cache_hit_rate ?? {
                  rate: null,
                  previous_rate: null,
                  change_pct: null,
                  trend: 'empty',
                },
              )}
            />
          </div>
        </div>
      )}
    </Card>
  );
}
