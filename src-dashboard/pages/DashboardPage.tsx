import { type ReactNode, useEffect, useState } from 'react';
import { Card, Col, Row, Table, Typography } from '@douyinfe/semi-ui';
import api from '../api';
import StatCard from '@components/dashboard/StatCard';
import TrendChart from '@components/dashboard/TrendChart';
import type { OverviewData, TopAccessPoint, TopModel, TrendItem } from '../types/dashboard.ts';
import { formatNumber } from '../utils/format.ts';

const {Title} = Typography;

// --- API 不可用时的 Mock 兜底数据 ---

const MOCK_OVERVIEW: OverviewData = {
  total_requests: 128456,
  total_requests_change: 12.5,
  active_access_points: 24,
  active_access_points_change: 8.3,
  active_users: 156,
  active_users_change: -2.1,
  error_rate: 2.3,
  error_rate_change: -0.5,
};

const MOCK_TRENDS: TrendItem[] = [
  {date: '05-13', count: 3200},
  {date: '05-14', count: 4100},
  {date: '05-15', count: 3800},
  {date: '05-16', count: 5200},
  {date: '05-17', count: 4900},
  {date: '05-18', count: 6100},
  {date: '05-19', count: 5800},
];

const MOCK_TOP_ACCESS_POINTS: TopAccessPoint[] = [
  {short_code: 'gp4', name: 'GPT-4 接入点', count: 45200},
  {short_code: 'cla3', name: 'Claude 3 接入点', count: 32100},
  {short_code: 'gemini', name: 'Gemini 接入点', count: 19800},
  {short_code: 'glm4', name: 'GLM-4 接入点', count: 12400},
  {short_code: 'qwen', name: '通义千问接入点', count: 8900},
];

const MOCK_TOP_MODELS: TopModel[] = [
  {model: 'gpt-4-turbo', count: 28500},
  {model: 'gpt-3.5-turbo', count: 22100},
  {model: 'claude-3-opus-20240229', count: 15300},
  {model: 'claude-3-sonnet-20240229', count: 12100},
  {model: 'gemini-1.5-pro', count: 9800},
];

// --- Top-N 表格列定义 ---

const ACCESS_POINT_COLUMNS = [
  {
    title: '排名',
    width: 60,
    align: 'center' as const,
    render: (_: unknown, __: unknown, idx: number) => idx + 1,
  },
  {title: '短码', dataIndex: 'short_code'},
  {title: '名称', dataIndex: 'name'},
  {
    title: '请求量',
    dataIndex: 'count',
    align: 'right' as const,
    render: (v: number) => formatNumber(v),
  },
];

const MODEL_COLUMNS = [
  {
    title: '排名',
    width: 60,
    align: 'center' as const,
    render: (_: unknown, __: unknown, idx: number) => idx + 1,
  },
  {title: '模型', dataIndex: 'model'},
  {
    title: '使用次数',
    dataIndex: 'count',
    align: 'right' as const,
    render: (v: number) => formatNumber(v),
  },
];

// --- 主页面组件 ---

/**
 * DashboardPage - Dashboard 概览页面
 *
 * 展示系统核心统计数据：总请求量、活跃接入点、活跃用户、错误率，
 * 近 7 天请求趋势图、Top-N 接入点和模型排名。
 * API 未就绪时回退到 Mock 数据用于 UI 演示。
 */
export default function DashboardPage(): ReactNode {
  const [loading, setLoading] = useState(true);
  const [overview, setOverview] = useState<OverviewData | null>(null);
  const [trends, setTrends] = useState<TrendItem[]>([]);
  const [topAccessPoints, setTopAccessPoints] = useState<TopAccessPoint[]>([]);
  const [topModels, setTopModels] = useState<TopModel[]>([]);

  useEffect(() => {
    let cancelled = false;

    const fetchData = async () => {
      try {
        const [ov, tr, ap, md] = await Promise.all([
          api.get<OverviewData>('/api/stats/overview'),
          api.get<TrendItem[]>('/api/stats/trends?days=7'),
          api.get<TopAccessPoint[]>('/api/stats/top-access-points?limit=5'),
          api.get<TopModel[]>('/api/stats/top-models?limit=5'),
        ]);
        if (cancelled) return;
        setOverview(ov);
        setTrends(tr);
        setTopAccessPoints(ap);
        setTopModels(md);
      } catch {
        console.warn('[DashboardPage] Dashboard API 尚未就绪，回退到 Mock 数据');
        if (cancelled) return;
        setOverview(MOCK_OVERVIEW);
        setTrends(MOCK_TRENDS);
        setTopAccessPoints(MOCK_TOP_ACCESS_POINTS);
        setTopModels(MOCK_TOP_MODELS);
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    fetchData();
    return () => { cancelled = true; };
  }, []);

  return (
    <div>
      <Title heading={3} style={{marginBottom: 24}}>
        Dashboard
      </Title>

      {/* ---- 统计卡片行 ---- */}
      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="总请求量 (近 30 天)"
            value={overview ? formatNumber(overview.total_requests) : '-'}
            change={overview?.total_requests_change}
            loading={loading}
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="活跃接入点"
            value={overview?.active_access_points ?? '-'}
            change={overview?.active_access_points_change}
            loading={loading}
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="活跃用户"
            value={overview?.active_users ?? '-'}
            change={overview?.active_users_change}
            loading={loading}
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="错误率"
            value={
              overview && overview.error_rate !== undefined
                ? `${overview.error_rate}%`
                : '-'
            }
            change={overview?.error_rate_change}
            loading={loading}
            invertChange
          />
        </Col>
      </Row>

      {/* ---- 趋势图 ---- */}
      <div style={{marginTop: 24}}>
        <TrendChart data={trends} loading={loading}/>
      </div>

      {/* ---- Top-N 表格 ---- */}
      <Row gutter={[16, 16]} style={{marginTop: 24}}>
        <Col xs={24} lg={12}>
          <Card
            title="Top 5 接入点"
            style={{backgroundColor: 'var(--semi-color-bg-0)'}}
          >
            <Table
              columns={ACCESS_POINT_COLUMNS}
              dataSource={topAccessPoints}
              pagination={false}
              size="small"
              loading={loading}
              rowKey="short_code"
            />
          </Card>
        </Col>
        <Col xs={24} lg={12}>
          <Card
            title="Top 5 模型"
            style={{backgroundColor: 'var(--semi-color-bg-0)'}}
          >
            <Table
              columns={MODEL_COLUMNS}
              dataSource={topModels}
              pagination={false}
              size="small"
              loading={loading}
              rowKey="model"
            />
          </Card>
        </Col>
      </Row>
    </div>
  );
}
