import { useState, useEffect, useCallback, useMemo, type ReactNode } from 'react';
import {
  Button, Typography, Select, Input,
} from '@douyinfe/semi-ui';
import { IconRefresh } from '@douyinfe/semi-icons';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import api from '../api.ts';
import LogFilterBar from '../components/LogFilterBar.tsx';
import RequestLogTable from '../components/RequestLogTable.tsx';
import type {
  AccessPointItem,
  LogFilters,
  LogSummary,
  PaginatedResult,
  UserItem,
} from '../types/log.ts';
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

      <RequestLogTable
        logs={logs}
        loading={loading}
        total={total}
        page={page}
        pageSize={pageSize}
        userMap={userMap}
        apMap={apMap}
        onPageChange={handlePageChange}
      />
    </div>
  );
}
