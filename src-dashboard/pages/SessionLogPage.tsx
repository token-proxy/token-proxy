import { useState, useEffect, useCallback, useMemo, type ReactNode } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Table, Button, Tag, Typography, Toast, Spin, Empty,
  Tooltip,
} from '@douyinfe/semi-ui';
import { IconRefresh } from '@douyinfe/semi-icons';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import api from '../api.ts';
import ClaudeSessionTimeline from '../components/ClaudeSessionTimeline.tsx';
import LogFilterBar from '../components/LogFilterBar.tsx';
import RawContentModal from '../components/RawContentModal.tsx';
import type {
  AccessPointItem,
  ConversationEvent,
  LogDetail,
  PaginatedResult,
  SessionListFilters,
  SessionSummary,
  UserItem,
} from '../types/log.ts';
import { formatDateTime, truncate, truncateMiddle } from '../utils/format.ts';
import { buildQueryString, toIsoString } from '../utils/query.ts';

const { Title, Text } = Typography;

// ─── Component ───

export default function SessionLogPage(): ReactNode {
  const { sessionId } = useParams<{ sessionId: string }>();
  const navigate = useNavigate();

  // Reference data for lookup maps
  const [users, setUsers] = useState<UserItem[]>([]);
  const [accessPoints, setAccessPoints] = useState<AccessPointItem[]>([]);

  // List mode state
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [sessionsLoading, setSessionsLoading] = useState(false);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const [filters, setFilters] = useState<SessionListFilters>({});

  // Detail mode state
  const [sessionEvents, setSessionEvents] = useState<ConversationEvent[]>([]);
  const [detailLoading, setDetailLoading] = useState(false);
  const [rawModalVisible, setRawModalVisible] = useState(false);
  const [rawModalTitle, setRawModalTitle] = useState('');
  const [rawModalContent, setRawModalContent] = useState('');

  // ─── Load reference data ───

  useEffect(() => {
    api.get<UserItem[]>('/api/users')
      .then(setUsers)
      .catch(() => {});
    api.get<AccessPointItem[]>('/api/access-points')
      .then(setAccessPoints)
      .catch(() => {});
  }, []);

  // ─── Lookup maps ───

  const userMap = useMemo(() => {
    const m: Record<string, string> = {};
    users.forEach((u) => { m[u.id] = u.display_name; });
    return m;
  }, [users]);

  const apMap = useMemo(() => {
    const m: Record<string, string> = {};
    accessPoints.forEach((ap) => { m[ap.id] = ap.name; });
    return m;
  }, [accessPoints]);

  // ─── Load sessions (list mode) ───

  const fetchSessions = useCallback(async () => {
    setSessionsLoading(true);
    try {
      const qs = buildQueryString({
        page,
        page_size: pageSize,
        start_time: filters.startTime,
        end_time: filters.endTime,
        user_id: filters.userId,
        access_point_id: filters.accessPointId,
      });
      const result = await api.get<PaginatedResult<SessionSummary>>(`/api/logs/sessions?${qs}`);
      setSessions(result.items);
      setTotal(result.total);
    } catch {
      setSessions([]);
      setTotal(0);
    } finally {
      setSessionsLoading(false);
    }
  }, [page, pageSize, filters]);

  useEffect(() => {
    if (!sessionId) {
      fetchSessions();
    }
  }, [sessionId, fetchSessions]);

  // ─── Load session detail ───

  const loadSessionDetail = useCallback(async (sid: string) => {
    setDetailLoading(true);
    try {
      const events = await api.get<ConversationEvent[]>(
        `/api/logs/sessions/${encodeURIComponent(sid)}`,
      );
      setSessionEvents(events);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '加载会话详情失败');
      setSessionEvents([]);
    } finally {
      setDetailLoading(false);
    }
  }, []);

  useEffect(() => {
    if (sessionId) {
      loadSessionDetail(sessionId);
    }
    return () => {
      setSessionEvents([]);
    };
  }, [sessionId, loadSessionDetail]);

  // ─── Modal helpers ───

  const openRawModal = async (logId: string) => {
    setRawModalTitle('原始日志内容');
    setRawModalContent('加载中...');
    setRawModalVisible(true);
    try {
      const detail = await api.get<LogDetail>(`/api/logs/${logId}/raw`);
      setRawModalContent([
        '=== 请求头 ===',
        JSON.stringify(detail.request_headers, null, 2),
        '',
        '=== 请求体 ===',
        JSON.stringify(detail.request_body, null, 2),
        '',
        '=== 响应体 ===',
        detail.response_body || '(空)',
      ].join('\n'));
    } catch (err) {
      setRawModalContent(err instanceof Error ? err.message : '加载原始内容失败');
    }
  };

  const closeRawModal = () => {
    setRawModalVisible(false);
  };

  // ─── Filter handlers ───

  const handleDateChange: DatePickerProps['onChange'] = (value) => {
    if (Array.isArray(value) && value.length === 2) {
      setFilters((prev) => ({
        ...prev,
        startTime: value[0] ? toIsoString(value[0]) : undefined,
        endTime: value[1] ? toIsoString(value[1]) : undefined,
      }));
    } else {
      setFilters((prev) => ({ ...prev, startTime: undefined, endTime: undefined }));
    }
  };

  const handleResetFilters = () => {
    setFilters({});
    setPage(1);
  };

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
  };

  // ─── Detail View ───

  if (sessionId) {
    const sortedEvents = [...sessionEvents].sort((a, b) => {
      if (a.request_index !== b.request_index) return a.request_index - b.request_index;
      return a.event_index - b.event_index;
    });

    return (
      <div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
          <Button type="tertiary" onClick={() => navigate('/sessions')}>
            &larr; 返回会话列表
          </Button>
          <Title heading={3} style={{ margin: 0 }}>会话详情</Title>
          <Button
            icon={<IconRefresh />}
            loading={detailLoading}
            onClick={() => loadSessionDetail(sessionId)}
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
          <ClaudeSessionTimeline events={sortedEvents} onOpenRaw={openRawModal} />
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
              render: (t: string) => formatDateTime(t),
            },
            {
              title: '来源',
              key: 'source',
              width: 120,
              render: (_: unknown, r: ConversationEvent) => (
                <Tag color={r.source === 'subagent' ? 'green' : 'blue'}>
                  {r.source === 'subagent' ? '子代理' : '主代理'}
                </Tag>
              ),
            },
            {
              title: '类型',
              dataIndex: 'event_type',
              width: 160,
            },
            {
              title: '摘要',
              key: 'summary',
              render: (_: unknown, r: ConversationEvent) => truncate(r.content_preview || r.title || r.content || '', 100),
            },
            {
              title: '操作',
              key: 'actions',
              width: 120,
              render: (_: unknown, r: ConversationEvent) => (
                <Button size="small" type="tertiary" onClick={() => openRawModal(r.log_id)}>
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
          onClose={closeRawModal}
        />
      </div>
    );
  }

  // ─── List View ───

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
        <Title heading={3} style={{ margin: 0 }}>会话日志</Title>
        <Button
          icon={<IconRefresh />}
          loading={sessionsLoading}
          onClick={() => fetchSessions()}
        >
          刷新
        </Button>
      </div>

      <LogFilterBar
        users={users.map((user) => ({ id: user.id, label: user.display_name }))}
        accessPoints={accessPoints.map((accessPoint) => ({ id: accessPoint.id, label: accessPoint.name }))}
        userId={filters.userId}
        accessPointId={filters.accessPointId}
        datePickerWidth={360}
        selectWidth={160}
        onDateChange={handleDateChange}
        onUserChange={(userId) => setFilters((prev) => ({ ...prev, userId }))}
        onAccessPointChange={(accessPointId) =>
          setFilters((prev) => ({ ...prev, accessPointId }))
        }
        onReset={handleResetFilters}
      />

      {/* Session Table */}
      <Table
        columns={[
          {
            title: '会话 ID',
            dataIndex: 'session_id',
            key: 'session_id',
            width: 180,
            render: (id: string) => (
              <Tooltip content={id}>
                <span style={{ fontFamily: 'monospace', fontSize: 13 }}>
                  {truncateMiddle(id)}
                </span>
              </Tooltip>
            ),
          },
          {
            title: '用户',
            key: 'user',
            width: 120,
            render: (_: unknown, r: SessionSummary) =>
              r.user_id ? (userMap[r.user_id] || r.user_id) : '-',
          },
          {
            title: '接入点',
            key: 'ap',
            width: 120,
            render: (_: unknown, r: SessionSummary) =>
              r.access_point_id ? (apMap[r.access_point_id] || r.access_point_id) : '-',
          },
          {
            title: '开始时间',
            dataIndex: 'start_time',
            width: 180,
            render: (t: string) => formatDateTime(t),
          },
          {
            title: '请求次数',
            dataIndex: 'request_count',
            width: 80,
          },
          {
            title: '首条摘要',
            dataIndex: 'first_message',
            render: (msg?: string | null) => truncate(msg ?? '', 80),
          },
          {
            title: '操作',
            key: 'actions',
            width: 100,
            render: (_: unknown, r: SessionSummary) => (
              <Button
                size="small"
                onClick={(e) => {
                  e.stopPropagation();
                  navigate(`/sessions/${encodeURIComponent(r.session_id)}`);
                }}
              >
                查看详情
              </Button>
            ),
          },
        ]}
        dataSource={sessions}
        loading={sessionsLoading}
        rowKey="session_id"
        scroll={{ x: 'max-content' }}
        pagination={{
          currentPage: page,
          pageSize,
          total,
          onChange: handlePageChange,
        }}
        onRow={(record?: SessionSummary) => record ? ({
          onClick: () => navigate(`/sessions/${encodeURIComponent(record.session_id)}`),
          style: { cursor: 'pointer' },
        }) : {}}
        empty={
          <Empty description={sessionsLoading ? '' : '暂无会话数据'} />
        }
      />
    </div>
  );
}
