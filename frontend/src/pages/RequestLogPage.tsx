import { useState, useEffect, useCallback, useMemo, type ReactNode } from 'react';
import {
  Table, Button, Tag, Typography, Toast, Modal, Empty,
  DatePicker, Select, Input,
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

interface LogFilters {
  startTime?: string;
  endTime?: string;
  sessionId?: string;
  userId?: string;
  accessPointId?: string;
  statusCode?: number | null;
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

function truncateMiddle(str: string | null | undefined, maxLen = 24): string {
  if (!str) return '-';
  if (str.length <= maxLen) return str;
  const half = Math.floor((maxLen - 3) / 2);
  return str.slice(0, half) + '...' + str.slice(-half);
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

const STATUS_OPTIONS = [
  { value: 200, label: '200' },
  { value: 201, label: '201' },
  { value: 400, label: '400' },
  { value: 401, label: '401' },
  { value: 403, label: '403' },
  { value: 404, label: '404' },
  { value: 429, label: '429' },
  { value: 500, label: '500' },
  { value: 502, label: '502' },
  { value: 503, label: '503' },
];

// ─── Component ───

export default function RequestLogPage(): ReactNode {
  // Reference data
  const [users, setUsers] = useState<UserItem[]>([]);
  const [accessPoints, setAccessPoints] = useState<AccessPointItem[]>([]);

  // List state
  const [logs, setLogs] = useState<LogSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [filters, setFilters] = useState<LogFilters>({});

  // Detail modal state
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailModalVisible, setDetailModalVisible] = useState(false);
  const [detailData, setDetailData] = useState<LogDetail | null>(null);

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

  // ─── Load logs ───

  const fetchLogs = useCallback(async () => {
    setLoading(true);
    try {
      const qs = buildQueryString({
        page,
        page_size: pageSize,
        start_time: filters.startTime,
        end_time: filters.endTime,
        session_id: filters.sessionId,
        user_id: filters.userId,
        access_point_id: filters.accessPointId,
        status_code: filters.statusCode,
      });
      const result = await api.get<PaginatedResult<LogSummary>>(`/api/logs?${qs}`);
      setLogs(result.items);
      setTotal(result.total);
    } catch {
      setLogs([]);
      setTotal(0);
    } finally {
      setLoading(false);
    }
  }, [page, pageSize, filters]);

  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  // ─── Detail modal ───

  const openDetail = async (id: string) => {
    setDetailLoading(true);
    setDetailModalVisible(true);
    setDetailData(null);
    try {
      const data = await api.get<LogDetail>(`/api/logs/${id}`);
      setDetailData(data);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '加载日志详情失败');
      setDetailModalVisible(false);
    } finally {
      setDetailLoading(false);
    }
  };

  const closeDetail = () => {
    setDetailModalVisible(false);
    setDetailData(null);
  };

  // ─── Filter handlers ───

  const handleDateChange: DatePickerProps['onChange'] = (value) => {
    if (Array.isArray(value) && value.length === 2) {
      setFilters((prev) => ({
        ...prev,
        startTime: value[0] ? value[0].toISOString() : undefined,
        endTime: value[1] ? value[1].toISOString() : undefined,
      }));
    } else {
      setFilters((prev) => ({ ...prev, startTime: undefined, endTime: undefined }));
    }
  };

  const handleReset = () => {
    setFilters({});
    setPage(1);
  };

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
  };

  // ─── Table columns ───

  const columns = [
    {
      title: '时间',
      dataIndex: 'timestamp',
      width: 180,
      render: (t: string) => formatDateTime(t),
    },
    {
      title: '会话 ID',
      dataIndex: 'session_id',
      width: 160,
      render: (id: string) => (
        <span style={{ fontFamily: 'monospace', fontSize: 12 }}>
          {truncateMiddle(id, 20)}
        </span>
      ),
    },
    {
      title: '用户',
      key: 'user',
      width: 100,
      render: (_: unknown, r: LogSummary) =>
        r.user_id ? (userMap[r.user_id] || truncateMiddle(r.user_id, 8)) : '-',
    },
    {
      title: '接入点',
      key: 'ap',
      width: 100,
      render: (_: unknown, r: LogSummary) =>
        r.access_point_id ? (apMap[r.access_point_id] || truncateMiddle(r.access_point_id, 8)) : '-',
    },
    {
      title: '原始模型',
      dataIndex: 'model_original',
      width: 120,
      render: (m?: string | null) => m || '-',
    },
    {
      title: '映射模型',
      dataIndex: 'model_mapped',
      width: 120,
      render: (m?: string | null) => m || '-',
    },
    {
      title: '状态码',
      dataIndex: 'status_code',
      width: 80,
      render: (code?: number | null) => (
        <Tag color={(code ?? 0) >= 400 ? 'red' : 'green'}>
          {code ?? '-'}
        </Tag>
      ),
    },
    {
      title: '耗时',
      dataIndex: 'duration_ms',
      width: 80,
      render: (ms?: number | null) => formatDuration(ms),
    },
    {
      title: '操作',
      key: 'actions',
      width: 80,
      render: (_: unknown, record: LogSummary) => (
        <Button
          size="small"
          onClick={(e) => {
            e.stopPropagation();
            openDetail(record.id);
          }}
        >
          详情
        </Button>
      ),
    },
  ];

  return (
    <div>
      <Title heading={3} style={{ marginBottom: 16 }}>请求日志</Title>

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
            style={{ width: 340 }}
          />
        </div>
        <div>
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>会话 ID</Text>
          <Input
            placeholder="输入会话 ID"
            value={filters.sessionId}
            onChange={(v: string) =>
              setFilters((prev) => ({ ...prev, sessionId: v || undefined }))
            }
            style={{ width: 180 }}
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
            style={{ width: 140 }}
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
            style={{ width: 140 }}
            showClear
          >
            {accessPoints.map((ap) => (
              <Select.Option key={ap.id} value={ap.id}>{ap.name}</Select.Option>
            ))}
          </Select>
        </div>
        <div>
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>状态码</Text>
          <Select
            placeholder="选择状态码"
            value={filters.statusCode}
            onChange={(v) =>
              setFilters((prev) => ({
                ...prev,
                statusCode: v == null ? undefined : Number(v),
              }))
            }
            style={{ width: 100 }}
            showClear
          >
            {STATUS_OPTIONS.map((opt) => (
              <Select.Option key={opt.value} value={opt.value}>{opt.label}</Select.Option>
            ))}
          </Select>
        </div>
        <Button onClick={handleReset}>重置</Button>
      </div>

      {/* Log Table */}
      <Table
        columns={columns}
        dataSource={logs}
        loading={loading}
        rowKey="id"
        scroll={{ x: 'max-content' }}
        pagination={{
          currentPage: page,
          pageSize,
          total,
          onChange: handlePageChange,
        }}
        empty={
          <Empty description={loading ? '' : '暂无日志数据'} />
        }
      />

      {/* Detail Modal */}
      <Modal
        title="请求详情"
        visible={detailModalVisible}
        onCancel={closeDetail}
        onOk={closeDetail}
        width={900}
        style={{ maxHeight: '80vh' }}
        footer={
          <Button type="primary" onClick={closeDetail}>关闭</Button>
        }
      >
        {detailLoading ? (
          <div style={{ textAlign: 'center', padding: 40 }}>
            <Text type="secondary">加载中...</Text>
          </div>
        ) : detailData ? (
          <div>
            {/* Meta info */}
            <div
              style={{
                background: 'var(--semi-color-fill-0)',
                borderRadius: 8,
                padding: 12,
                marginBottom: 16,
                display: 'flex',
                flexDirection: 'column',
                gap: 6,
                fontSize: 13,
              }}
            >
              <Text>
                <strong>时间:</strong> {formatDateTime(detailData.timestamp)}
              </Text>
              <Text>
                <strong>会话 ID:</strong>
                {' '}
                <span style={{ fontFamily: 'monospace', fontSize: 12 }}>
                  {detailData.session_id}
                </span>
              </Text>
              <Text>
                <strong>模型:</strong> {detailData.model_original || '-'}
                {' '}
                &rarr;
                {' '}
                {detailData.model_mapped || '-'}
              </Text>
              <Text>
                <strong>状态码:</strong>
                {' '}
                <Tag
                  color={(detailData.status_code ?? 0) >= 400 ? 'red' : 'green'}
                  size="small"
                >
                  {detailData.status_code ?? '-'}
                </Tag>
                <strong style={{ marginLeft: 16 }}>耗时:</strong>
                {' '}
                {formatDuration(detailData.duration_ms)}
              </Text>
              {detailData.error_message && (
                <Text>
                  <strong style={{ color: 'var(--semi-color-danger)' }}>错误:</strong>
                  {' '}
                  {detailData.error_message}
                </Text>
              )}
            </div>

            {/* Request Headers */}
            <Text strong style={{ display: 'block', marginBottom: 4 }}>请求头:</Text>
            <pre
              style={{
                background: 'var(--semi-color-fill-0)',
                padding: 12,
                borderRadius: 4,
                fontSize: 12,
                overflow: 'auto',
                maxHeight: 200,
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-all',
              }}
            >
              {JSON.stringify(detailData.request_headers, null, 2) || '(空)'}
            </pre>

            {/* Request Body */}
            <Text strong style={{ display: 'block', marginTop: 12, marginBottom: 4 }}>请求体:</Text>
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
              {JSON.stringify(detailData.request_body, null, 2) || '(空)'}
            </pre>

            {/* Response Body */}
            <Text strong style={{ display: 'block', marginTop: 12, marginBottom: 4 }}>响应体:</Text>
            <pre
              style={{
                background: 'var(--semi-color-fill-0)',
                padding: 12,
                borderRadius: 4,
                fontSize: 12,
                overflow: 'auto',
                maxHeight: 400,
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-all',
              }}
            >
              {detailData.response_body || '(空)'}
            </pre>
          </div>
        ) : (
          <Empty description="暂无数据" />
        )}
      </Modal>
    </div>
  );
}
