import type { ReactNode } from 'react';
import {
  Button, Table, Tag, Typography, Spin, Empty,
} from '@douyinfe/semi-ui';
import { IconRefresh } from '@douyinfe/semi-icons';
import ClaudeSessionTimeline from './ClaudeSessionTimeline.tsx';
import RawContentModal from './RawContentModal.tsx';
import type { ConversationEvent, TokenUsage } from '../types/log.ts';
import { formatDateTime, truncate } from '../utils/format.ts';

const { Title, Text } = Typography;

interface SessionDetailViewProps {
  sessionId: string;
  sortedEvents: ConversationEvent[];
  tokenUsageMap: Record<string, TokenUsage>;
  detailLoading: boolean;
  onBack: () => void;
  onRefresh: () => void;
  onOpenRaw: (logId: string) => void;
  rawModalVisible: boolean;
  rawModalTitle: string;
  rawModalContent: string;
  onCloseRawModal: () => void;
}

export default function SessionDetailView({
  sessionId,
  sortedEvents,
  tokenUsageMap,
  detailLoading,
  onBack,
  onRefresh,
  onOpenRaw,
  rawModalVisible,
  rawModalTitle,
  rawModalContent,
  onCloseRawModal,
}: SessionDetailViewProps): ReactNode {
  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
        <Button type="tertiary" onClick={onBack}>
          &larr; 返回会话列表
        </Button>
        <Title heading={3} style={{ margin: 0 }}>会话详情</Title>
        <Button
          icon={<IconRefresh />}
          loading={detailLoading}
          onClick={onRefresh}
        >
          刷新
        </Button>
      </div>

      <div
        style={{
          background: 'var(--semi-color-fill-0)',
          borderRadius: 8,
          padding: 16,
          marginBottom: 24,
        }}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          <Text>
            <strong>会话 ID:</strong>{' '}
            <span style={{ fontFamily: 'monospace', fontSize: 13 }}>{sessionId}</span>
          </Text>
          <Text><strong>事件总数:</strong> {sortedEvents.length}</Text>
          {sortedEvents.length > 0 && (
            <Text>
              <strong>时间范围:</strong>{' '}
              {formatDateTime(sortedEvents[0].timestamp)} ~ {formatDateTime(sortedEvents[sortedEvents.length - 1].timestamp)}
            </Text>
          )}
        </div>
      </div>

      <Title heading={6} style={{ marginBottom: 16 }}>对话内容</Title>
      {detailLoading ? (
        <div style={{ textAlign: 'center', padding: 40 }}>
          <Spin />
          <Text type="secondary" style={{ display: 'block', marginTop: 8 }}>加载对话内容中...</Text>
        </div>
      ) : sortedEvents.length === 0 ? (
        <Empty description="暂无对话数据" />
      ) : (
        <ClaudeSessionTimeline events={sortedEvents} onOpenRaw={onOpenRaw} tokenUsageMap={tokenUsageMap} />
      )}

      <Title heading={6} style={{ marginBottom: 16 }}>事件摘要</Title>
      <Table
        columns={[
          {
            title: '序号',
            key: 'index',
            width: 70,
            render: (_: unknown, _r: ConversationEvent, i: number) => i + 1,
          },
          {
            title: '时间',
            dataIndex: 'timestamp',
            width: 180,
            render: (t: string) => <span style={{ whiteSpace: 'nowrap' }}>{formatDateTime(t)}</span>,
          },
          {
            title: '来源',
            key: 'source',
            width: 120,
            render: (_: unknown, r: ConversationEvent) => (
              <span style={{ whiteSpace: 'nowrap' }}>
                <Tag color={r.source === 'subagent' ? 'green' : 'blue'}>
                  {r.source === 'subagent' ? '子代理' : '主代理'}
                </Tag>
              </span>
            ),
          },
          {
            title: '类型',
            dataIndex: 'event_type',
            width: 160,
            render: (t: string) => <span style={{ whiteSpace: 'nowrap' }}>{t}</span>,
          },
          {
            title: '摘要',
            key: 'summary',
            render: (_: unknown, r: ConversationEvent) => truncate(r.title || r.content || '', 100),
          },
          {
            title: '操作',
            key: 'actions',
            width: 120,
            render: (_: unknown, r: ConversationEvent) => (
              <Button size="small" type="tertiary" onClick={() => onOpenRaw(r.log_id)}>
                原始内容
              </Button>
            ),
          },
        ]}
        dataSource={sortedEvents}
        rowKey="id"
        loading={detailLoading}
        size="small"
        scroll={{ x: 'max-content' }}
        pagination={false}
      />

      <RawContentModal
        title={rawModalTitle}
        visible={rawModalVisible}
        content={rawModalContent}
        onClose={onCloseRawModal}
      />
    </div>
  );
}
