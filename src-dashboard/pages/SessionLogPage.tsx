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
import CopyableIdText from '../components/CopyableIdText.tsx';
import LogFilterBar from '../components/LogFilterBar.tsx';
import RawContentModal from '../components/RawContentModal.tsx';
import type {
  AccessPointItem,
  ConversationEvent,
  LogDetail,
  PaginatedResult,
  SessionContentItem,
  SessionListFilters,
  SessionSummary,
  TokenUsage,
  UserItem,
} from '../types/log.ts';
import { formatDateTime, truncate } from '../utils/format.ts';
import { buildConversationEvents } from '../utils/parseLogs.ts';
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
  const [sessionTokenUsage, setSessionTokenUsage] = useState<TokenUsage[]>([]);
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
      const contents = await api.get<SessionContentItem[]>(
        `/api/logs/sessions/${encodeURIComponent(sid)}/contents`,
      );
      // 客户端构建 ConversationEvent[]
      const events: ConversationEvent[] = [];
      for (const item of contents) {
        events.push(...buildConversationEvents(
          item.request_body,
          item.response_body,
          {
            log_id: item.log_id,
            timestamp: item.timestamp,
            request_index: item.request_index,
            conversation_source: item.conversation_source,
            agent_id: item.agent_id ?? undefined,
          },
        ));
      }
      setSessionEvents(events);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '加载会话详情失败');
      setSessionEvents([]);
    } finally {
      setDetailLoading(false);
    }
  }, []);

  const fetchSessionTokenUsage = useCallback(async (sid: string) => {
    try {
      const usage = await api.get<TokenUsage[]>(
        `/api/logs/sessions/${encodeURIComponent(sid)}/token-usage`,
      );
      setSessionTokenUsage(usage);
    } catch {
      setSessionTokenUsage([]);
    }
  }, []);

  useEffect(() => {
    if (sessionId) {
      loadSessionDetail(sessionId);
      fetchSessionTokenUsage(sessionId);
    }
    return () => {
      setSessionEvents([]);
      setSessionTokenUsage([]);
    };
  }, [sessionId, loadSessionDetail, fetchSessionTokenUsage]);

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

  // ─── Token usage map ───

  const tokenUsageMap = useMemo(() => {
    const m: Record<string, TokenUsage> = {};
    sessionTokenUsage.forEach((tu) => { m[tu.log_id] = tu; });
    return m;
  }, [sessionTokenUsage]);

  // ─── Detail View ───

  if (sessionId) {
    const sortedEvents = [...sessionEvents].sort((a, b) => {
      if (a.request_index !== b.request_index) return a.request_index - b.request_index;
      if (a.event_index !== b.event_index) return a.event_index - b.event_index;
      const timeCmp = new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime();
      if (timeCmp !== 0) return timeCmp;
      return a.id.localeCompare(b.id);
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
            onClick={() => {
              if (sessionId) {
                loadSessionDetail(sessionId);
                fetchSessionTokenUsage(sessionId);
              }
            }}
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
          <ClaudeSessionTimeline events={sortedEvents} onOpenRaw={openRawModal} tokenUsageMap={tokenUsageMap} />
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
            width: 280,
            render: (id: string) => (
              <CopyableIdText value={id} />
            ),
          },
          {
            title: '用户',
            key: 'user',
            width: 100,
            render: (_: unknown, r: SessionSummary) => {
              if (!r.user_id) return '-';
              const name = userMap[r.user_id];
              return name
                ? <span style={{ whiteSpace: 'nowrap' }}>{name}</span>
                : <CopyableIdText value={r.user_id} />;
            },
          },
          {
            title: <span>接<br />入点</span>,
            key: 'ap',
            width: 110,
            render: (_: unknown, r: SessionSummary) => {
              if (!r.access_point_id) return '-';
              const name = apMap[r.access_point_id];
              return name
                ? <Text ellipsis style={{ maxWidth: 110 }}>{name}</Text>
                : <CopyableIdText value={r.access_point_id} />;
            },
          },
          {
            title: '开始时间',
            dataIndex: 'start_time',
            width: 180,
            render: (t: string) => <span style={{ whiteSpace: 'nowrap' }}>{formatDateTime(t)}</span>,
          },
          {
            title: <span>请求次<br />数</span>,
            dataIndex: 'request_count',
            width: 90,
            render: (v: number) => (
              <span style={{ whiteSpace: 'nowrap', display: 'block', maxWidth: 90 }}>{v}</span>
            ),
          },
          {
            title: 'Token',
            key: 'token',
            width: 140,
            render: (_: unknown, record: SessionSummary) => {
              const hasToken = record.total_input_tokens > 0 || record.total_output_tokens > 0;
              if (!hasToken) return <span style={{ color: 'var(--semi-color-text-2)' }}>-</span>;
              return (
                <Tooltip
                  content={
                    <div style={{ fontSize: 12, lineHeight: 1.6 }}>
                      <div>总输入: {record.total_input_tokens.toLocaleString()}</div>
                      <div>总输出: {record.total_output_tokens.toLocaleString()}</div>
                      <div>缓存创建: {record.total_cache_creation_input_tokens.toLocaleString()}</div>
                      <div>缓存读取: {record.total_cache_read_input_tokens.toLocaleString()}</div>
                      <div>思考: {record.total_thinking_tokens.toLocaleString()}</div>
                      <div>总计: {record.total_tokens.toLocaleString()}</div>
                    </div>
                  }
                >
                  <span style={{ whiteSpace: 'nowrap', cursor: 'default' }}>
                    &uarr;{record.total_input_tokens.toLocaleString()} / &darr;{record.total_output_tokens.toLocaleString()}
                  </span>
                </Tooltip>
              );
            },
          },
          {
            title: '操作',
            key: 'actions',
            width: 100,
              render: (_: unknown, r: SessionSummary) => (
              <Button
                size="small"
                onClick={() => {
                  window.open(
                    `/sessions/${encodeURIComponent(r.session_id)}`,
                    '_blank',
                    'noopener',
                  );
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
        empty={
          <Empty description={sessionsLoading ? '' : '暂无会话数据'} />
        }
      />
    </div>
  );
}
