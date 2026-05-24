import { useState, useEffect, useCallback, useMemo, type ReactNode } from 'react';
import {
  Table, Button, Tag, Typography, Toast, Empty,
  Select, Input,
} from '@douyinfe/semi-ui';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import api from '../api.ts';
import LogDetailModal from '../components/LogDetailModal.tsx';
import LogFilterBar from '../components/LogFilterBar.tsx';
import type {
  AccessPointItem,
  LogDetail,
  LogFilters,
  LogSummary,
  PaginatedResult,
  UserItem,
} from '../types/log.ts';
import { formatDateTime, formatDuration, truncateMiddle } from '../utils/format.ts';
import { buildQueryString, toIsoString } from '../utils/query.ts';

const { Title, Text } = Typography;

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
  const [pageSize] = useState(20);
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
        startTime: value[0] ? toIsoString(value[0]) : undefined,
        endTime: value[1] ? toIsoString(value[1]) : undefined,
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
          loading={detailLoading}
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

      <LogFilterBar
        users={users.map((user) => ({ id: user.id, label: user.display_name }))}
        accessPoints={accessPoints.map((accessPoint) => ({ id: accessPoint.id, label: accessPoint.name }))}
        userId={filters.userId}
        accessPointId={filters.accessPointId}
        onDateChange={handleDateChange}
        onUserChange={(userId) => setFilters((prev) => ({ ...prev, userId }))}
        onAccessPointChange={(accessPointId) =>
          setFilters((prev) => ({ ...prev, accessPointId }))
        }
        onReset={handleReset}
      >
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
      </LogFilterBar>

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

      <LogDetailModal
        visible={detailModalVisible}
        loading={detailLoading}
        data={detailData}
        onClose={closeDetail}
      />
    </div>
  );
}
