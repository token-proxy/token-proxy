import { useState, useEffect, useCallback, useMemo, type ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Table, Button, Tag, Typography, Empty,
  Select, Input, Tooltip,
} from '@douyinfe/semi-ui';
import { IconRefresh } from '@douyinfe/semi-icons';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import api from '../api.ts';
import CopyableIdText from '../components/CopyableIdText.tsx';
import LogFilterBar from '../components/LogFilterBar.tsx';
import type {
  AccessPointItem,
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

  // ─── Navigation ───
  const navigate = useNavigate();

  // ─── Filter handlers ───

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
      title: 'ID',
      dataIndex: 'id',
      width: 240,
      render: (id: string) => <CopyableIdText value={id} />,
    },
    {
      title: '时间',
      dataIndex: 'timestamp',
      width: 180,
      render: (t: string) => formatDateTime(t),
    },
    {
      title: '对话内容',
      dataIndex: 'message_preview',
      width: 360,
      render: (_: unknown, r: LogSummary) => (
        <Text
          ellipsis
          style={{ maxWidth: 360 }}
        >
          {r.message_preview || '-'}
        </Text>
      ),
    },
    {
      title: '来源',
      key: 'source',
      width: 140,
      render: (_: unknown, r: LogSummary) => (
        <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
          <Tag color={r.conversation_source === 'subagent' ? 'green' : 'blue'}>
            {r.conversation_source === 'subagent' ? '子代理' : '主代理'}
          </Tag>
          {r.agent_type && <Tag>{r.agent_type}</Tag>}
          {r.primary_tool_name && <Tag color="violet">{r.primary_tool_name}</Tag>}
        </div>
      ),
    },
    {
      title: '会话 ID',
      dataIndex: 'session_id',
      width: 260,
      render: (id: string) => <CopyableIdText value={id} />,
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
      width: 180,
      render: (_: unknown, r: LogSummary) => {
        if (!r.access_point_id) {
          return '-';
        }

        const accessPointName = apMap[r.access_point_id];
        if (accessPointName) {
          return <span className="monospace-text nowrap-text">{accessPointName}</span>;
        }

        return <CopyableIdText value={r.access_point_id} />;
      },
    },
    {
      title: '原始模型',
      dataIndex: 'model_original',
      width: 120,
      render: (m?: string | null) => <span className="nowrap-text">{m || '-'}</span>,
    },
    {
      title: '映射模型',
      dataIndex: 'model_mapped',
      width: 120,
      render: (m?: string | null) => <span className="nowrap-text">{m || '-'}</span>,
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
      title: 'Token',
      key: 'token',
      width: 140,
      render: (_: unknown, record: LogSummary) => {
        const hasToken = record.token_input_tokens != null || record.token_output_tokens != null;
        if (!hasToken) return <span style={{ color: 'var(--semi-color-text-2)' }}>-</span>;
        const input = record.token_input_tokens?.toLocaleString() || '0';
        const output = record.token_output_tokens?.toLocaleString() || '0';
        const total = record.token_total_tokens?.toLocaleString() || '0';
        return (
          <Tooltip
            content={
              <div style={{ fontSize: 12, lineHeight: 1.6 }}>
                <div>输入 token：{input}</div>
                <div>输出 token：{output}</div>
                <div>总计：{total}</div>
              </div>
            }
          >
            <span style={{ whiteSpace: 'nowrap', cursor: 'default' }}>
              ↑{input} / ↓{output}
            </span>
          </Tooltip>
        );
      },
    },
    {
      title: '耗时',
      dataIndex: 'duration_ms',
      width: 80,
      render: (ms?: number | null) => <span className="nowrap-text">{formatDuration(ms)}</span>,
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
            navigate(`/logs/${record.id}`);
          }}
        >
          详情
        </Button>
      ),
    },
  ];

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
        <Title heading={3} style={{ margin: 0 }}>请求日志</Title>
        <Button
          icon={<IconRefresh />}
          loading={loading}
          onClick={() => fetchLogs()}
        >
          刷新
        </Button>
      </div>

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

    </div>
  );
}
