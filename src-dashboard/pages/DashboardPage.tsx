/**
 * Dashboard 主页面。
 *
 * 面向技术主管的数据洞察视图，组合 F3 ~ F9 交付的所有 dashboard 组件：
 * - 顶部时间范围切换器（全局过滤器 + 刷新按钮）
 * - 4 张 KPI 卡（总请求数 / Token 总量 / 活跃成员数 / 缓存命中率）
 * - 双列排行（成员请求量 Top 10 / 账号 Token 消耗 Top 10）
 *
 * 设计风格参照 Linear Insights / Vercel Analytics 的极简数据洞察布局。
 */

import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import { Notification, Typography } from '@douyinfe/semi-ui';
import { dashboardApi } from '../api';
import { CacheHitCard } from '../components/dashboard/CacheHitCard';
import { KpiCard } from '../components/dashboard/KpiCard';
import { TimeRangeSelector } from '../components/dashboard/TimeRangeSelector';
import { TopAccountsRanking } from '../components/dashboard/TopAccountsRanking';
import { TopUsersRanking } from '../components/dashboard/TopUsersRanking';
import { useFetch } from '../hooks/useFetch';
import type { TimeRangeQuery } from '../types/dashboard';
import { formatNumber, formatTokenCompact } from '../utils/format';
import './DashboardPage.css';

/**
 * Dashboard 主页面组件。
 *
 * 顶层管理 timeRange 和 refreshKey 两个状态，通过 useFetch 并行加载 3 个数据源：
 * - KPI（含 4 张卡 + sparkline 时间序列）
 * - 成员排行 Top 10
 * - 账号排行 Top 10
 *
 * 刷新策略：refreshKey 自增触发 useFetch 重新执行，避免 useFetch 内部 deps 不变时跳过。
 * 错误处理：3 个查询任一失败时通过 useEffect 单次弹出 Notification，避免每次 render 重复弹。
 */
export default function DashboardPage(): ReactNode {
  // 默认范围：近 7 天（与 TimeRangeSelector 的 last7 预设对齐）
  const [timeRange, setTimeRange] = useState<TimeRangeQuery>({ range: 'last7' });
  // refreshKey 自增触发 useFetch 重新执行；时间范围切换本身已会触发 fetch，仅刷新按钮使用
  const [refreshKey, setRefreshKey] = useState(0);

  // useFetch 的 deps 通过 useMemo 稳定引用，避免每次 render 创建新数组
  // 拆为基础字段而非整个 timeRange 对象，确保浅比较生效
  const fetchDeps = useMemo(
    () => [timeRange.range, timeRange.start, timeRange.end, refreshKey],
    [timeRange.range, timeRange.start, timeRange.end, refreshKey],
  );

  const kpiQuery = useFetch(() => dashboardApi.getKpi(timeRange), fetchDeps);
  const topUsersQuery = useFetch(() => dashboardApi.getTopUsers(timeRange), fetchDeps);
  const topAccountsQuery = useFetch(() => dashboardApi.getTopAccounts(timeRange), fetchDeps);

  /** 刷新按钮回调：refreshKey 自增以触发 useFetch 重新执行 */
  const handleRefresh = useCallback(() => {
    setRefreshKey((k) => k + 1);
  }, []);

  // 错误聚合：3 个查询任一失败即视为页面级错误（useFetch 返回 error 为 string | null）
  const error = kpiQuery.error ?? topUsersQuery.error ?? topAccountsQuery.error;

  // 错误状态变化时单次弹通知；放在 useEffect 内防止每次 render 重复弹
  useEffect(() => {
    if (error) {
      Notification.error({
        title: 'Dashboard 数据加载失败',
        content: error,
        duration: 5,
      });
    }
  }, [error]);

  // 任一查询加载中即整体显示加载（驱动刷新按钮 spinner）
  const isLoading = kpiQuery.loading || topUsersQuery.loading || topAccountsQuery.loading;

  // 从 KPI 响应中提取三条 sparkline 序列，缺失时回退空数组
  const sparklineBuckets = kpiQuery.data?.sparkline.buckets ?? [];
  const sparklineRequests = sparklineBuckets.map((b) => b.request_count);
  const sparklineTokens = sparklineBuckets.map((b) => b.total_tokens);
  const sparklineUsers = sparklineBuckets.map((b) => b.active_user_count);

  return (
    <div className="dashboard-container">
      {/* 顶部：标题 + 时间范围切换器 */}
      <div className="dashboard-header">
        <Typography.Title heading={3} style={{ margin: 0 }}>
          数据洞察
        </Typography.Title>
        <TimeRangeSelector
          value={timeRange}
          onChange={setTimeRange}
          onRefresh={handleRefresh}
          loading={isLoading}
        />
      </div>

      {/* KPI 卡片区：4 列网格，窄屏自动降级为 2 列 / 1 列 */}
      <div className="dashboard-grid-kpi">
        <KpiCard
          title="总请求数"
          value={kpiQuery.data?.request_count.current ?? 0}
          format={formatNumber}
          trend={kpiQuery.data?.request_count.trend ?? 'empty'}
          changePct={kpiQuery.data?.request_count.change_pct ?? null}
          sparklineData={sparklineRequests}
          loading={kpiQuery.loading}
        />
        <KpiCard
          title="Token 总量"
          value={kpiQuery.data?.total_tokens.current ?? 0}
          format={formatTokenCompact}
          trend={kpiQuery.data?.total_tokens.trend ?? 'empty'}
          changePct={kpiQuery.data?.total_tokens.change_pct ?? null}
          sparklineData={sparklineTokens}
          loading={kpiQuery.loading}
        />
        <KpiCard
          title="活跃成员数"
          value={kpiQuery.data?.active_user_count.current ?? 0}
          format={formatNumber}
          trend={kpiQuery.data?.active_user_count.trend ?? 'empty'}
          changePct={kpiQuery.data?.active_user_count.change_pct ?? null}
          sparklineData={sparklineUsers}
          loading={kpiQuery.loading}
        />
        <CacheHitCard
          rate={kpiQuery.data?.cache_hit_rate.rate ?? null}
          trend={kpiQuery.data?.cache_hit_rate.trend ?? 'empty'}
          changePct={kpiQuery.data?.cache_hit_rate.change_pct ?? null}
          loading={kpiQuery.loading}
        />
      </div>

      {/* 排行区：双列布局，窄屏降级为单列 */}
      <div className="dashboard-grid-rank">
        <TopUsersRanking items={topUsersQuery.data?.items ?? []} loading={topUsersQuery.loading} />
        <TopAccountsRanking
          items={topAccountsQuery.data?.items ?? []}
          loading={topAccountsQuery.loading}
        />
      </div>
    </div>
  );
}
