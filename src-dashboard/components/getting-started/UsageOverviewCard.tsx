/**
 * 用量总览卡片组件。
 *
 * 封装热力图数据获取与渲染，自包含 useFetch、错误通知、刷新按钮和 Card 包装。
 * 指标切换 ButtonGroup 置于 Card 标题行，与右侧 MetricsCard 的 TimeRangeSelector 对称。
 */

import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { Button, ButtonGroup, Card, Notification } from '@douyinfe/semi-ui';
import { IconRefresh } from '@douyinfe/semi-icons';
import { dashboardApi } from '../../api';
import { browserTimezone, type HeatmapMetric } from './heatmapUtils';
import { useFetch } from '../../hooks/useFetch';
import { Heatmap } from './Heatmap';

/**
 * 用量总览卡片：近 1 年每日用量热力图。
 *
 * 数据依赖：`GET /api/getting-started/heatmap`（当前自然年，仅受刷新按钮影响）。
 */
export function UsageOverviewCard(): ReactNode {
  const tz = useMemo(() => browserTimezone(), []);
  const heatmapDeps = useMemo(() => [], []);
  const [metric, setMetric] = useState<HeatmapMetric>('tokens');

  const { data, loading, error, refetch } = useFetch(
    () => dashboardApi.getHeatmap(tz),
    heatmapDeps,
  );

  // 错误通知
  useEffect(() => {
    if (error) {
      Notification.error({ title: '用量数据加载失败', content: error, duration: 5 });
    }
  }, [error]);

  return (
    <Card
      title="用量总览"
      className="gs-hero-left-card"
      headerExtraContent={
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
          <ButtonGroup size="small" aria-label="热力图单位">
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
          <Button
            icon={<IconRefresh />}
            loading={loading}
            onClick={refetch}
            type="tertiary"
            size="small"
          >
            刷新
          </Button>
        </div>
      }
    >
      <Heatmap cells={data?.cells ?? []} loading={loading} metric={metric} />
    </Card>
  );
}
