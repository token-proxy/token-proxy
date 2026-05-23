import { useState, useEffect, useCallback, useMemo, type ReactNode } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Table, Button, Tag, Typography, Toast, Modal, Spin, Empty,
  DatePicker, Select, Tooltip,
} from '@douyinfe/semi-ui';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import api from '../api.ts';

const { Title, Text } = Typography;

// ─── Types ───

interface UserItem {
  id: string;
  username: string;
  display_name: string;
  status: string;
}

interface AccessPointItem {
  id: string;
  name: string;
  short_code: string;
}

interface SessionSummary {
  session_id: string;
  user_id?: string | null;
  access_point_id?: string | null;
  start_time: string;
  request_count: number;
  first_message?: string | null;
}

interface LogSummary {
  id: string;
  timestamp: string;
  session_id: string;
  user_id?: string | null;
  access_point_id?: string | null;
  model_original?: string | null;
  model_mapped?: string | null;
  status_code?: number | null;
  duration_ms?: number | null;
}

interface LogDetail {
  id: string;
  timestamp: string;
  session_id: string;
  user_id?: string | null;
  access_point_id?: string | null;
  provider_id?: string | null;
  account_id?: string | null;
  model_original?: string | null;
  model_mapped?: string | null;
  status_code?: number | null;
  duration_ms?: number | null;
  error_message?: string | null;
  request_headers?: Record<string, unknown> | null;
  request_body?: Record<string, unknown> | null;
  response_body?: string | null;
}

interface PaginatedResult<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
}

interface SessionListFilters {
  startTime?: string;
  endTime?: string;
  userId?: string;
  accessPointId?: string;
}

// ─── Helpers ───

function formatDateTime(ts: string | null | undefined): string {
  if (!ts) return '-';
  try {
    return new Date(ts).toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  } catch {
    return ts;
  }
}

function formatDuration(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) return '-';
  return `${ms} ms`;
}

function truncate(str: string | null | undefined, maxLen: number): string {
  if (!str) return '-';
  return str.length > maxLen ? str.slice(0, maxLen) + '...' : str;
}

function truncateMiddle(str: string | null | undefined, maxLen = 16): string {
  if (!str) return '-';
  if (str.length <= maxLen) return str;
  const half = Math.floor((maxLen - 3) / 2);
  return str.slice(0, half) + '...' + str.slice(-half);
}

function toIsoString(value: string | Date): string {
  return value instanceof Date ? value.toISOString() : new Date(value).toISOString();
}

function buildQueryString(params: Record<string, string | number | boolean | null | undefined>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '' && value !== false) {
      search.set(key, String(value));
    }
  }
  return search.toString();
}

function extractLastUserMessage(requestBody?: Record<string, unknown> | null): string {
  if (!requestBody) return '(无请求体)';
  const body = requestBody as Record<string, unknown>;
  const messages = body.messages;
  if (Array.isArray(messages)) {
    const userMsgs = messages.filter((m: Record<string, unknown>) => m.role === 'user');
    if (userMsgs.length > 0) {
      const last = userMsgs[userMsgs.length - 1] as Record<string, unknown>;
      if (typeof last.content === 'string') return last.content;
      if (Array.isArray(last.content)) {
        return (last.content as Array<Record<string, unknown>>)
          .map((c: Record<string, unknown>) => typeof c.text === 'string' ? c.text : '')
          .filter(Boolean)
          .join('');
      }
      return JSON.stringify(last.content);
    }
  }
  // Fallback: present the whole request body
  return JSON.stringify(requestBody, null, 2);
}

function extractTextFromSSE(sse: string): string {
  const lines = sse.split('\n');
  const texts: string[] = [];

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith('data:')) {
      const data = trimmed.slice(5).trim();
      if (data === '[DONE]') continue;

      try {
        const parsed = JSON.parse(data) as Record<string, unknown>;
        const choices = parsed.choices;
        if (Array.isArray(choices)) {
          for (const c of choices as Array<Record<string, unknown>>) {
            const delta = c.delta as Record<string, unknown> | undefined;
            if (delta?.content && typeof delta.content === 'string') texts.push(delta.content);
            else if (delta?.text && typeof delta.text === 'string') texts.push(delta.text);
            else if (c.text && typeof c.text === 'string') texts.push(c.text);
          }
        } else if (parsed.type === 'content_block_delta') {
          const delta = parsed.delta as Record<string, unknown> | undefined;
          if (delta?.text && typeof delta.text === 'string') texts.push(delta.text);
        } else if (parsed.content && typeof parsed.content === 'string') {
          texts.push(parsed.content);
        }
      } catch {
        texts.push(data);
      }
    }
  }

  return texts.join('');
}

function extractAssistantResponse(responseBody?: string | null): string {
  if (!responseBody) return '(无响应体)';

  // Try direct JSON parse (non-streaming response)
  try {
    const parsed = JSON.parse(responseBody) as Record<string, unknown>;
    const choices = parsed.choices;
    if (Array.isArray(choices)) {
      const text = (choices as Array<Record<string, unknown>>)
        .map((c: Record<string, unknown>) => {
          const msg = c.message as Record<string, unknown> | undefined;
          if (msg?.content && typeof msg.content === 'string') return msg.content;
          if (c.text && typeof c.text === 'string') return c.text;
          return '';
        })
        .filter(Boolean)
        .join('');
      if (text) return text;
    }
    const content = parsed.content;
    if (Array.isArray(content)) {
      const text = (content as Array<Record<string, unknown>>)
        .map((c: Record<string, unknown>) => typeof c.text === 'string' ? c.text : '')
        .filter(Boolean)
        .join('');
      if (text) return text;
    }
    if (typeof content === 'string') return content;

    return JSON.stringify(parsed, null, 2);
  } catch {
    // Handle SSE streaming response
    const extracted = extractTextFromSSE(responseBody);
    return extracted || responseBody;
  }
}

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

        {/* Session Info Header */}
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
              <strong>会话 ID:</strong>
              {' '}
              <span style={{ fontFamily: 'monospace', fontSize: 13 }}>{sessionId}</span>
            </Text>
            <Text><strong>请求总数:</strong> {sessionLogs.length}</Text>
            {sessionLogs.length > 0 && (
              <>
                <Text>
                  <strong>时间范围:</strong>
                  {' '}
                  {formatDateTime(sessionLogs[0].timestamp)}
                  {' '}
                  ~
                  {' '}
                  {formatDateTime(sessionLogs[sessionLogs.length - 1].timestamp)}
                </Text>
                <Text>
                  <strong>用户:</strong>
                  {' '}
                  {(() => {
                    const uid = sessionLogs[0].user_id;
                    return uid ? (userMap[uid] || uid) : '-';
                  })()}
                </Text>
              </>
            )}
          </div>
        </div>

        {/* Chat Bubble View */}
        <Title heading={6} style={{ marginBottom: 16 }}>对话内容</Title>
        {detailLoading ? (
          <div style={{ textAlign: 'center', padding: 40 }}>
            <Spin />
            <Text type="secondary" style={{ display: 'block', marginTop: 8 }}>加载对话内容中...</Text>
          </div>
        ) : sortedDetails.length === 0 ? (
          <Empty description="暂无对话数据" />
        ) : (
          <div style={{ marginBottom: 32 }}>
            {sortedDetails.map((detail, index) => {
              const userMsg = extractLastUserMessage(detail.request_body);
              const assistantMsg = extractAssistantResponse(detail.response_body);
              return (
                <div
                  key={detail.id}
                  style={{
                    border: '1px solid var(--semi-color-border)',
                    borderRadius: 8,
                    marginBottom: 16,
                    padding: 16,
                  }}
                >
                  <div style={{ marginBottom: 12 }}>
                    <Tag color="blue" size="small">第 {index + 1} 轮</Tag>
                  </div>

                  {/* User Message */}
                  <div
                    style={{
                      background: 'var(--semi-color-primary-light-default)',
                      borderRadius: 8,
                      padding: '8px 12px',
                      marginBottom: 12,
                      maxWidth: '85%',
                    }}
                  >
                    <Text type="secondary" style={{ fontSize: 12 }}>用户</Text>
                    <div
                      style={{
                        marginTop: 4,
                        whiteSpace: 'pre-wrap',
                        wordBreak: 'break-word',
                        fontSize: 14,
                        lineHeight: 1.6,
                      }}
                    >
                      {userMsg}
                    </div>
                  </div>

                  {/* Assistant Response */}
                  <div
                    style={{
                      background: 'var(--semi-color-fill-0)',
                      borderRadius: 8,
                      padding: '8px 12px',
                      marginBottom: 12,
                      maxWidth: '85%',
                      marginLeft: 'auto',
                    }}
                  >
                    <Text type="secondary" style={{ fontSize: 12 }}>助手</Text>
                    <div
                      style={{
                        marginTop: 4,
                        whiteSpace: 'pre-wrap',
                        wordBreak: 'break-word',
                        fontSize: 14,
                        lineHeight: 1.6,
                      }}
                    >
                      {assistantMsg}
                    </div>
                  </div>

                  {/* Metadata */}
                  <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', alignItems: 'center', marginBottom: 8 }}>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {formatDateTime(detail.timestamp)}
                    </Text>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      <Tag size="small">{detail.model_original || '-'}</Tag>
                      {' '}
                      &rarr;
                      {' '}
                      <Tag size="small" color="blue">{detail.model_mapped || '-'}</Tag>
                    </Text>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {formatDuration(detail.duration_ms)}
                    </Text>
                    <Tag
                      color={(detail.status_code ?? 0) >= 400 ? 'red' : 'green'}
                      size="small"
                    >
                      {detail.status_code ?? '-'}
                    </Tag>
                  </div>

                  <Button
                    size="small"
                    type="tertiary"
                    onClick={() => {
                      openRawModal(
                        `第 ${index + 1} 轮原始内容`,
                        [
                          '=== 请求体 ===',
                          JSON.stringify(detail.request_body, null, 2),
                          '',
                          '=== 响应体 ===',
                          detail.response_body || '(空)',
                        ].join('\n'),
                      );
                    }}
                  >
                    展开原始内容
                  </Button>
                </div>
              );
            })}
          </div>
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

        {/* Raw Content Modal */}
        <Modal
          title={rawModalTitle}
          visible={rawModalVisible}
          onCancel={closeRawModal}
          onOk={closeRawModal}
          width={800}
          style={{ maxHeight: '80vh' }}
        >
          <pre
            style={{
              background: 'var(--semi-color-fill-0)',
              padding: 16,
              borderRadius: 4,
              fontSize: 12,
              overflow: 'auto',
              maxHeight: 500,
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-all',
            }}
          >
            {rawModalContent}
          </pre>
        </Modal>
      </div>
    );
  }

  // ─── List View ───

  return (
    <div>
      <Title heading={3} style={{ marginBottom: 16 }}>会话日志</Title>

      {/* Filter Bar */}
      <div
        style={{
          display: 'flex',
          gap: 12,
          marginBottom: 16,
          flexWrap: 'wrap',
          alignItems: 'flex-end',
        }}
      >
        <div>
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>时间范围</Text>
          <DatePicker
            type="dateTimeRange"
            onChange={handleDateChange}
            style={{ width: 360 }}
          />
        </div>
        <div>
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>用户</Text>
          <Select
            placeholder="选择用户"
            value={filters.userId}
            onChange={(v) =>
              setFilters((prev) => ({
                ...prev,
                userId: v == null ? undefined : String(v),
              }))
            }
            style={{ width: 160 }}
            showClear
          >
            {users.map((u) => (
              <Select.Option key={u.id} value={u.id}>{u.display_name}</Select.Option>
            ))}
          </Select>
        </div>
        <div>
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>接入点</Text>
          <Select
            placeholder="选择接入点"
            value={filters.accessPointId}
            onChange={(v) =>
              setFilters((prev) => ({
                ...prev,
                accessPointId: v == null ? undefined : String(v),
              }))
            }
            style={{ width: 160 }}
            showClear
          >
            {accessPoints.map((ap) => (
              <Select.Option key={ap.id} value={ap.id}>{ap.name}</Select.Option>
            ))}
          </Select>
        </div>
        <Button onClick={handleResetFilters}>重置</Button>
      </div>

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
