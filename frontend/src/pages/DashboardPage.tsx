import { useState, useEffect, type ReactNode } from 'react';
import { Card, Row, Col, Table, Typography, Tag, Spin } from '@douyinfe/semi-ui';
import { IconArrowUp, IconArrowDown, IconMinus } from '@douyinfe/semi-icons';
import api from '../api';

const { Title, Text } = Typography;

// --- Types for statistics data ---

interface OverviewData {
  total_requests: number;
  total_requests_change: number;
  active_access_points: number;
  active_access_points_change: number;
  active_users: number;
  active_users_change: number;
  error_rate: number;
  error_rate_change: number;
}

interface TrendItem {
  date: string;
  count: number;
}

interface TopAccessPoint {
  short_code: string;
  name: string;
  count: number;
}

interface TopModel {
  model: string;
  count: number;
}

// --- Mock data for fallback when API is unavailable ---

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
  { date: '05-13', count: 3200 },
  { date: '05-14', count: 4100 },
  { date: '05-15', count: 3800 },
  { date: '05-16', count: 5200 },
  { date: '05-17', count: 4900 },
  { date: '05-18', count: 6100 },
  { date: '05-19', count: 5800 },
];

const MOCK_TOP_ACCESS_POINTS: TopAccessPoint[] = [
  { short_code: 'gp4', name: 'GPT-4 接入点', count: 45200 },
  { short_code: 'cla3', name: 'Claude 3 接入点', count: 32100 },
  { short_code: 'gemini', name: 'Gemini 接入点', count: 19800 },
  { short_code: 'glm4', name: 'GLM-4 接入点', count: 12400 },
  { short_code: 'qwen', name: '通义千问接入点', count: 8900 },
];

const MOCK_TOP_MODELS: TopModel[] = [
  { model: 'gpt-4-turbo', count: 28500 },
  { model: 'gpt-3.5-turbo', count: 22100 },
  { model: 'claude-3-opus-20240229', count: 15300 },
  { model: 'claude-3-sonnet-20240229', count: 12100 },
  { model: 'gemini-1.5-pro', count: 9800 },
];

// --- Helper functions ---

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}

/**
 * Shows a colored tag indicating change percentage.
 * @param invert - When true, flips green/red (used for error rate where lower is better).
 */
function ChangeIndicator({ value, invert }: { value: number; invert?: boolean }) {
  const adjusted = invert ? -value : value;
  const color = adjusted > 0 ? 'green' : adjusted < 0 ? 'red' : 'grey';
  const Icon = adjusted > 0 ? IconArrowUp : adjusted < 0 ? IconArrowDown : IconMinus;
  const label = value > 0 ? `+${value}%` : value < 0 ? `${value}%` : '0%';

  return (
    <Tag
      color={color}
      style={{ marginLeft: 8, display: 'inline-flex', alignItems: 'center', gap: 2 }}
    >
      <Icon size="small" />
      {label}
    </Tag>
  );
}

/**
 * A single stat card with title, large value, and change indicator.
 */
function StatCard({
  title,
  value,
  change,
  loading,
  invertChange,
}: {
  title: string;
  value: string | number;
  change: number;
  loading: boolean;
  invertChange?: boolean;
}) {
  return (
    <Card style={{ minHeight: 140, backgroundColor: 'var(--semi-color-bg-0)' }}>
      <Text type="secondary" style={{ fontSize: 14 }}>
        {title}
      </Text>
      {loading ? (
        <div style={{ marginTop: 20 }}>
          <Spin size="small" />
        </div>
      ) : (
        <>
          <Title heading={3} style={{ marginTop: 8, marginBottom: 4 }}>
            {value}
          </Title>
          <ChangeIndicator value={change} invert={invertChange} />
        </>
      )}
    </Card>
  );
}

/**
 * Simple CSS-only bar chart for request trends.
 */
function TrendChart({ data, loading }: { data: TrendItem[]; loading: boolean }) {
  const maxCount = Math.max(...data.map((d) => d.count), 1);
  const barMaxHeight = 180;

  return (
    <Card
      title="请求趋势 (近 7 天)"
      style={{ backgroundColor: 'var(--semi-color-bg-0)' }}
    >
      {loading ? (
        <div style={{ textAlign: 'center', padding: '60px 0' }}>
          <Spin />
        </div>
      ) : (
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-end',
            gap: 12,
            paddingTop: 24,
            paddingBottom: 4,
          }}
        >
          {data.map((item) => {
            const barHeight = Math.max((item.count / maxCount) * barMaxHeight, 4);
            return (
              <div
                key={item.date}
                style={{
                  flex: 1,
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                }}
              >
                <Text
                  type="secondary"
                  style={{ fontSize: 11, marginBottom: 4, whiteSpace: 'nowrap' }}
                >
                  {item.count >= 1000
                    ? `${(item.count / 1000).toFixed(1)}K`
                    : item.count}
                </Text>
                <div
                  style={{
                    width: '100%',
                    maxWidth: 40,
                    height: barHeight,
                    backgroundColor: 'var(--semi-color-primary)',
                    borderRadius: '4px 4px 0 0',
                    transition: 'height 0.3s ease',
                    opacity: 0.85,
                  }}
                />
                <Text
                  type="secondary"
                  style={{ fontSize: 12, marginTop: 8 }}
                >
                  {item.date}
                </Text>
              </div>
            );
          })}
        </div>
      )}
    </Card>
  );
}

// --- Column definitions for top-N tables ---

const ACCESS_POINT_COLUMNS = [
  {
    title: '排名',
    width: 60,
    align: 'center' as const,
    render: (_: unknown, __: unknown, idx: number) => idx + 1,
  },
  { title: '短码', dataIndex: 'short_code' },
  { title: '名称', dataIndex: 'name' },
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
  { title: '模型', dataIndex: 'model' },
  {
    title: '使用次数',
    dataIndex: 'count',
    align: 'right' as const,
    render: (v: number) => formatNumber(v),
  },
];

// --- Main page component ---

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
        // Fallback to mock data when API is not yet available
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
      <Title heading={3} style={{ marginBottom: 24 }}>
        Dashboard
      </Title>

      {/* ---- Stats cards row ---- */}
      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="总请求量 (近 30 天)"
            value={overview ? formatNumber(overview.total_requests) : '-'}
            change={overview?.total_requests_change ?? 0}
            loading={loading}
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="活跃接入点"
            value={overview?.active_access_points ?? '-'}
            change={overview?.active_access_points_change ?? 0}
            loading={loading}
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="活跃用户"
            value={overview?.active_users ?? '-'}
            change={overview?.active_users_change ?? 0}
            loading={loading}
          />
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <StatCard
            title="错误率"
            value={overview ? `${overview.error_rate}%` : '-'}
            change={overview?.error_rate_change ?? 0}
            loading={loading}
            invertChange
          />
        </Col>
      </Row>

      {/* ---- Trend chart ---- */}
      <div style={{ marginTop: 24 }}>
        <TrendChart data={trends} loading={loading} />
      </div>

      {/* ---- Top-N tables ---- */}
      <Row gutter={[16, 16]} style={{ marginTop: 24 }}>
        <Col xs={24} lg={12}>
          <Card
            title="Top 5 接入点"
            style={{ backgroundColor: 'var(--semi-color-bg-0)' }}
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
            style={{ backgroundColor: 'var(--semi-color-bg-0)' }}
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
