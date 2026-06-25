/**
 * 用量总览卡片组件。
 *
 * 封装热力图数据获取与渲染，自包含 useFetch、错误通知、Card 包装。
 * 通过 `refreshKey` prop 接收外部刷新信号，独立于其他卡片的数据生命周期。
 */

import { useEffect, useMemo, type ReactNode } from 'react';
import { Card, Notification } from '@douyinfe/semi-ui';
import { dashboardApi } from '../../api';
import { browserTimezone } from './heatmapUtils';
import { useFetch } from '../../hooks/useFetch';
import { Heatmap } from './Heatmap';

/** UsageOverviewCard 组件 Props */
export interface UsageOverviewCardProps {
  /**
   * 刷新键：值变化时触发数据重新获取。
   * 由父组件通过自增 state 驱动，实现跨卡片统一刷新。
   */
  refreshKey: number;
}

/**
 * 用量总览卡片：近 1 年每日用量热力图。
 *
 * 数据依赖：`GET /api/getting-started/heatmap`（当前自然年，仅受 refreshKey 影响）。
 */
export function UsageOverviewCard({ refreshKey }: UsageOverviewCardProps): ReactNode {
  const tz = useMemo(() => browserTimezone(), []);
  const heatmapDeps = useMemo(() => [refreshKey], [refreshKey]);

  const { data, loading, error } = useFetch(() => dashboardApi.getHeatmap(tz), heatmapDeps);

  // 错误通知
  useEffect(() => {
    if (error) {
      Notification.error({ title: '用量数据加载失败', content: error, duration: 5 });
    }
  }, [error]);

  return (
    <Card title="用量总览" className="gs-hero-left-card">
      <Heatmap cells={data?.cells ?? []} loading={loading} />
    </Card>
  );
}
