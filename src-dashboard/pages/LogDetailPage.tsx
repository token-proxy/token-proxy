import { type ReactNode } from 'react';
import { useFetch } from '../hooks/useFetch.ts';
import { useNavigate, useParams } from 'react-router-dom';
import { Breadcrumb, Button, Card, Empty, Skeleton } from '@douyinfe/semi-ui';
import type { LogDetailFull } from '../types/log.ts';
import api from '../api.ts';
import BasicInfoCard from '@components/log/log-detail/BasicInfoCard';
import TokenUsageCard from '@components/log/log-detail/TokenUsageCard';
import HeadersCard from '@components/log/log-detail/HeadersCard.tsx';
import RequestContentCard from '@components/log/log-detail/RequestContentCard';
import ResponseContentCard from '@components/log/log-detail/ResponseContentCard';

/**
 * LogDetailPage - 日志详情页面
 *
 * 展示单条日志的完整详情：基础信息、Token 用量、请求头、请求内容解析、
 * 响应头、响应内容解析。支持加载中、错误、空数据三种状态展示。
 */
export default function LogDetailPage(): ReactNode {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const {
    data,
    loading,
    error,
    refetch: fetchDetail,
  } = useFetch(async () => {
    if (!id) throw new Error('日志 ID 不存在');
    return api.get<LogDetailFull>(`/api/logs/${id}`);
  }, [id]);

  // ── 加载中状态 ──

  if (loading) {
    return (
      <>
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
      </>
    );
  }

  // ── 错误状态 ──

  if (error) {
    return (
      <Empty description={error} style={{ marginTop: 80 }}>
        <Button type="primary" onClick={fetchDetail}>
          重试
        </Button>
      </Empty>
    );
  }

  if (!data) {
    return <Empty description="未找到日志详情" style={{ marginTop: 80 }} />;
  }

  // ── 正常渲染 ──

  return (
    <div>
      {/* 面包屑 */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
        <Breadcrumb>
          <Breadcrumb.Item>
            <span
              style={{ cursor: 'pointer', color: 'var(--semi-color-text-2)' }}
              onClick={() => navigate('/logs')}
            >
              请求日志
            </span>
          </Breadcrumb.Item>
          <Breadcrumb.Item>日志详情</Breadcrumb.Item>
        </Breadcrumb>
      </div>

      {/* 1. 基础信息 */}
      <BasicInfoCard data={data} style={{ marginBottom: 16 }} />

      {/* 2. Token 用量 */}
      <TokenUsageCard data={data} style={{ marginBottom: 16 }} />

      {/* 3. 请求头 */}
      <HeadersCard
        title="请求头"
        headers={data.request_headers as Record<string, unknown>}
        style={{ marginBottom: 16 }}
      />

      {/* 4. 请求内容 */}
      <RequestContentCard requestBody={data.request_body} style={{ marginBottom: 16 }} />

      {/* 5. 响应头 */}
      <HeadersCard
        title="响应头"
        headers={data.response_headers as Record<string, unknown>}
        style={{ marginBottom: 16 }}
      />

      {/* 6. 响应内容 */}
      <ResponseContentCard responseBody={data.response_body} />
    </div>
  );
}
