import { useState, useEffect, useCallback, type ReactNode } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Breadcrumb,
  Button,
  Card,
  Empty,
  Skeleton,
} from '@douyinfe/semi-ui';
import { IconArrowLeft } from '@douyinfe/semi-icons';
import type { LogDetailFull } from '../types/log.ts';
import api from '../api.ts';
import BasicInfoCard from '../components/BasicInfoCard.tsx';
import TokenUsageCard from '../components/TokenUsageCard.tsx';
import RequestHeadersCard from '../components/RequestHeadersCard.tsx';
import RequestContentCard from '../components/RequestContentCard.tsx';
import ResponseContentCard from '../components/ResponseContentCard.tsx';

export default function LogDetailPage(): ReactNode {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const [data, setData] = useState<LogDetailFull | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchDetail = useCallback(async () => {
    if (!id) {
      setError('日志 ID 不存在');
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const result = await api.get<LogDetailFull>(`/api/logs/${id}`);
      setData(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : '加载日志详情失败');
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    fetchDetail();
  }, [fetchDetail]);

  // ── 加载中状态 ──

  if (loading) {
    return (
      <div style={{ padding: 24 }}>
        <Skeleton
          active
          placeholder={<Skeleton.Title style={{ width: 200 }} />}
          style={{ marginBottom: 24 }}
        />
        <Card>
          <Skeleton active placeholder={<Skeleton.Paragraph rows={6} />} />
        </Card>
        <div style={{ marginTop: 16 }}>
          <Card>
            <Skeleton active placeholder={<Skeleton.Paragraph rows={4} />} />
          </Card>
        </div>
      </div>
    );
  }

  // ── 错误状态 ──

  if (error) {
    return (
      <div style={{ padding: 24 }}>
        <Empty description={error} style={{ marginTop: 80 }}>
          <Button type="primary" onClick={fetchDetail}>
            重试
          </Button>
        </Empty>
      </div>
    );
  }

  if (!data) {
    return (
      <div style={{ padding: 24 }}>
        <Empty description="未找到日志详情" style={{ marginTop: 80 }} />
      </div>
    );
  }

  // ── 正常渲染 ──

  return (
    <div style={{ padding: 24 }}>
      {/* 面包屑 */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
        <Breadcrumb>
          <Breadcrumb.Item>
            <span
              style={{ cursor: 'pointer', color: 'var(--semi-color-text-2)' }}
              onClick={() => navigate('/logs')}
            >
              日志列表
            </span>
          </Breadcrumb.Item>
          <Breadcrumb.Item>请求详情</Breadcrumb.Item>
        </Breadcrumb>
      </div>

      <div style={{ marginBottom: 20 }}>
        <Button
          icon={<IconArrowLeft />}
          type="tertiary"
          onClick={() => navigate('/logs')}
        >
          返回日志列表
        </Button>
      </div>

      {/* 1. 基础信息 */}
      <BasicInfoCard data={data} style={{ marginBottom: 16 }} />

      {/* 2. Token 用量 */}
      <TokenUsageCard data={data} style={{ marginBottom: 16 }} />

      {/* 3. 请求头 */}
      <RequestHeadersCard
        headers={data.request_headers as Record<string, unknown>}
        style={{ marginBottom: 16 }}
      />

      {/* 4. 请求内容 */}
      <RequestContentCard
        requestBody={data.request_body}
        style={{ marginBottom: 16 }}
      />

      {/* 5. 响应内容 */}
      <ResponseContentCard responseBody={data.response_body} />
    </div>
  );
}
