import { type ReactNode } from 'react';
import { useFetch } from '../hooks/useFetch.ts';
import { useNavigate, useParams } from 'react-router-dom';
import { Breadcrumb, Button, Card, Empty, Skeleton } from '@douyinfe/semi-ui';
import type { LogDetailFull } from '../types/log.ts';
import api from '../api.ts';
import BasicInfoCard from '@components/log/log-detail/BasicInfoCard';
import TokenUsageCard from '@components/log/log-detail/TokenUsageCard';
import TrajectoryView from '@components/log/log-detail/TrajectoryView';
import TrajectoryUnavailable from '@components/log/log-detail/TrajectoryUnavailable';
import { hasTrajectoryContent } from '../utils/trajectory.ts';

/**
 * LogDetailPage - 日志详情页面
 *
 * 展示单条日志：基础信息 → 词元用量 → 主视图。主视图按原文可用性二选一：
 * - 原文尚在 → 运行轨迹（工具栏 + 三泳道时间轴 + 轮次账本 + 记录检查器）
 * - 原文已被清理（请求体与响应体都没有）→ 运行轨迹不可用说明卡，不再画空轨迹
 * 支持加载中、错误、空数据三种状态展示。
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

      {/* 2. 词元用量 */}
      <TokenUsageCard data={data} style={{ marginBottom: 16 }} />

      {/* 3. 主视图：原文可用则画运行轨迹，原文已被清理则画说明卡
          （key 保证切换日志时重建选中与折叠状态） */}
      {hasTrajectoryContent(data) ? (
        <TrajectoryView key={data.id} data={data} />
      ) : (
        <TrajectoryUnavailable
          data={data}
          reason={data.content_available === false ? 'cleaned' : 'empty'}
        />
      )}
    </div>
  );
}
