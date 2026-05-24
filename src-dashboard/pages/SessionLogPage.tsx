import { useState, useEffect, useCallback, useMemo, type ReactNode } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Table, Button, Tag, Typography, Toast, Spin, Empty,
  Tooltip,
} from '@douyinfe/semi-ui';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import api from '../api.ts';
import ChatBubbleView from '../components/ChatBubbleView.tsx';
import LogFilterBar from '../components/LogFilterBar.tsx';
import RawContentModal from '../components/RawContentModal.tsx';
import SessionInfoHeader from '../components/SessionInfoHeader.tsx';
import type {
  AccessPointItem,
  LogDetail,
  LogSummary,
  PaginatedResult,
  SessionListFilters,
  SessionSummary,
  UserItem,
} from '../types/log.ts';
import { formatDateTime, formatDuration, truncate, truncateMiddle } from '../utils/format.ts';
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
  const [sessionLogs, setSessionLogs] = useState<LogSummary[]>([]);
  const [detailLoading, setDetailLoading] = useState(false);
  const [logDetails, setLogDetails] = useState<Record<string, LogDetail>>({});
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
      const summaries = await api.get<LogSummary[]>(
        `/api/logs/sessions/${encodeURIComponent(sid)}`,
      );
      setSessionLogs(summaries);

      // Load detail content in parallel
      const detailResults = await Promise.allSettled(
        summaries.map((s) => api.get<LogDetail>(`/api/logs/${s.id}`)),
      );
      const details: Record<string, LogDetail> = {};
      for (const result of detailResults) {
        if (result.status === 'fulfilled' && result.value) {
          details[result.value.id] = result.value;
        }
      }
      setLogDetails(details);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '加载会话详情失败');
      setSessionLogs([]);
      setLogDetails({});
    } finally {
      setDetailLoading(false);
    }
  }, []);

  useEffect(() => {
    if (sessionId) {
      loadSessionDetail(sessionId);
    }
    return () => {
      setSessionLogs([]);
      setLogDetails({});
    };
  }, [sessionId, loadSessionDetail]);

  // ─── Modal helpers ───

  const openRawModal = (title: string, content: string) => {
    setRawModalTitle(title);
    setRawModalContent(content);
    setRawModalVisible(true);
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
    const sortedDetails = sessionLogs
      .map((s) => logDetails[s.id])
      .filter((d): d is LogDetail => !!d)
      .sort(
        (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime(),
      );

    return (
      <div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
          <Button type="tertiary" onClick={() => navigate('/sessions')}>
            &larr; 返回会话列表
          </Button>
          <Title heading={3} style={{ margin: 0 }}>会话详情</Title>
        </div>

        <SessionInfoHeader
          sessionId={sessionId}
          sessionLogs={sessionLogs}
          userMap={userMap}
        />

        <Title heading={6} style={{ marginBottom: 16 }}>对话内容</Title>
        {detailLoading ? (
          <div style={{ textAlign: 'center', padding: 40 }}>
            <Spin />
            <Text type="secondary" style={{ display: 'block', marginTop: 8 }}>加载对话内容中...</Text>
          </div>
        ) : sortedDetails.length === 0 ? (
          <Empty description="暂无对话数据" />
        ) : (
          <ChatBubbleView details={sortedDetails} onOpenRaw={openRawModal} />
        )}

        {/* Request Rounds Table */}
        <Title heading={6} style={{ marginBottom: 16 }}>请求轮次</Title>
        <Table
          columns={[
            {
              title: '轮次',
              key: 'index',
              width: 60,
              render: (_: unknown, _r: LogDetail, i: number) => i + 1,
            },
            {
              title: '时间',
              dataIndex: 'timestamp',
              width: 180,
              render: (t: string) => formatDateTime(t),
            },
            {
              title: '模型',
              key: 'model',
              render: (_: unknown, r: LogDetail) =>
                `${r.model_original || '-'} → ${r.model_mapped || '-'}`,
            },
            {
              title: '状态码',
              dataIndex: 'status_code',
              width: 100,
              render: (code?: number | null) => (
                <Tag color={(code ?? 0) >= 400 ? 'red' : 'green'}>{code ?? '-'}</Tag>
              ),
            },
            {
              title: '耗时',
              dataIndex: 'duration_ms',
              width: 100,
              render: (ms?: number | null) => formatDuration(ms),
            },
          ]}
          dataSource={sortedDetails}
          rowKey="id"
          loading={detailLoading}
          size="small"
          scroll={{ x: 'max-content' }}
          pagination={false}
          expandedRowRender={(record?: LogDetail) => record ? (
            <div style={{ padding: 12 }}>
              <Text strong style={{ display: 'block', marginBottom: 4 }}>请求体:</Text>
              <pre
                style={{
                  background: 'var(--semi-color-fill-0)',
                  padding: 12,
                  borderRadius: 4,
                  fontSize: 12,
                  overflow: 'auto',
                  maxHeight: 300,
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-all',
                }}
              >
                {JSON.stringify(record.request_body, null, 2) || '(空)'}
              </pre>
              <Text strong style={{ display: 'block', marginTop: 12, marginBottom: 4 }}>响应体:</Text>
              <pre
                style={{
                  background: 'var(--semi-color-fill-0)',
                  padding: 12,
                  borderRadius: 4,
                  fontSize: 12,
                  overflow: 'auto',
                  maxHeight: 300,
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-all',
                }}
              >
                {record.response_body || '(空)'}
              </pre>
            </div>
          ) : null}
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
      <Title heading={3} style={{ marginBottom: 16 }}>会话日志</Title>

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
