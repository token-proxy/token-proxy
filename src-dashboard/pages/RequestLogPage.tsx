import { type ReactNode, useEffect, useMemo, useRef, useState } from 'react';
import { useFetch } from '../hooks/useFetch.ts';
import { useLogEvents } from '../hooks/useLogEvents';
import ConnectionIndicator from '@components/common/ConnectionIndicator';
import { Button, Input, Select, Typography } from '@douyinfe/semi-ui';
import { IconRefresh } from '@douyinfe/semi-icons';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import api from '../api.ts';
import LogFilterBar from '@components/log/LogFilterBar';
import RequestLogTable from '@components/log/RequestLogTable';
import type {
  AccessPointItem,
  LogFilters,
  LogSummary,
  PaginatedResult,
  UserItem,
} from '../types/log.ts';
import type { Account } from '@components/provider/AccountManager';
import { buildQueryString, toIsoString } from '../utils/query.ts';

/** 服务商简要信息 */
interface ProviderItem {
  id: string;
  name: string;
}

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

// ─── 组件 ───

/**
 * RequestLogPage - 请求日志列表页面
 *
 * 提供请求日志的分页浏览、多条件筛选（时间范围、用户、接入点、服务商、账号、中断、会话 ID、状态码）。
 */
export default function RequestLogPage(): ReactNode {
  // 参考数据
  const [users, setUsers] = useState<UserItem[]>([]);
  const [accessPoints, setAccessPoints] = useState<AccessPointItem[]>([]);
  const [providers, setProviders] = useState<ProviderItem[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);

  // 列表状态
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const [filters, setFilters] = useState<LogFilters>({});

  // ─── 筛选处理 ───

  useEffect(() => {
    api
      .get<UserItem[]>('/api/users')
      .then(setUsers)
      .catch(() => console.warn('[RequestLogPage] 加载用户参考数据失败'));
    api
      .get<AccessPointItem[]>('/api/access-points')
      .then(setAccessPoints)
      .catch(() => console.warn('[RequestLogPage] 加载接入点参考数据失败'));
    // 1. 先加载所有服务商
    api
      .get<ProviderItem[]>('/api/providers')
      .then((providerList) => {
        setProviders(providerList);
        // 2. 基于服务商列表并行拉取所有账号
        Promise.all(
          providerList.map((p) =>
            api.get<Account[]>(`/api/providers/${p.id}/accounts`).catch(() => [] as Account[]),
          ),
        )
          .then((results) => setAccounts(results.flat()))
          .catch(() => console.warn('[RequestLogPage] 加载账号参考数据失败'));
      })
      .catch(() => console.warn('[RequestLogPage] 加载服务商参考数据失败'));
  }, []);

  // ─── 查找映射 ───

  const userMap = useMemo(() => {
    const m: Record<string, string> = {};
    users.forEach((u) => {
      m[u.id] = u.display_name;
    });
    return m;
  }, [users]);

  const apMap = useMemo(() => {
    const m: Record<string, string> = {};
    accessPoints.forEach((ap) => {
      m[ap.id] = ap.name;
    });
    return m;
  }, [accessPoints]);

  const providerMap = useMemo(() => {
    const m: Record<string, string> = {};
    providers.forEach((p) => {
      m[p.id] = p.name;
    });
    return m;
  }, [providers]);

  const accountMap = useMemo(() => {
    const m: Record<string, string> = {};
    accounts.forEach((a) => {
      m[a.id] = a.name;
    });
    return m;
  }, [accounts]);

  // ─── 加载日志 ───

  const {
    data: logsData,
    loading,
    refetch: fetchLogs,
  } = useFetch(async () => {
    const qs = buildQueryString({
      page,
      page_size: pageSize,
      start_time: filters.startTime,
      end_time: filters.endTime,
      session_id: filters.sessionId,
      user_id: filters.userId,
      access_point_id: filters.accessPointId,
      status_code: filters.statusCode,
      provider_id: filters.providerId,
      account_id: filters.accountId,
      is_interrupted: filters.isInterrupted,
    });
    return api.get<PaginatedResult<LogSummary>>(`/api/logs?${qs}`);
  }, [page, pageSize, JSON.stringify(filters)]);
  const logs = logsData?.items ?? [];
  const total = logsData?.total ?? 0;

  // ─── 筛选处理 ───

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

  // ─── SSE 实时推送 ───

  const { status: sseStatus, lastEvent, onVisibilityRecover } = useLogEvents();

  // 翻页/有筛选时，累积未查看的新事件数
  const [pendingEventCount, setPendingEventCount] = useState(0);
  // 首页自动刷新后，短暂展示新增条数（null = 不展示）
  const [addedCount, setAddedCount] = useState<number | null>(null);
  // 自动刷新前记录当前 total，用于计算增量
  const preRefreshTotalRef = useRef(0);
  const isAutoRefreshingRef = useRef(false);

  // 判断是否在首页且无筛选（此时自动刷新不会影响用户浏览体验）
  const isFirstPageNoFilters = useMemo(
    () => page === 1 && Object.keys(filters).length === 0,
    [page, filters],
  );

  // 收到新日志事件时的行为
  useEffect(() => {
    if (!lastEvent) return;
    if (isFirstPageNoFilters) {
      preRefreshTotalRef.current = total;
      isAutoRefreshingRef.current = true;
      fetchLogs();
    } else {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setPendingEventCount((prev) => prev + 1);
    }
    // 依赖 lastEvent 更新时间戳，每次新事件触发
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lastEvent]);

  // 检测自动刷新完成，计算并展示增量
  useEffect(() => {
    if (!isAutoRefreshingRef.current) return;
    if (loading) return;
    const delta = total - preRefreshTotalRef.current;
    if (delta > 0) {
      setAddedCount(delta);
    }
    isAutoRefreshingRef.current = false;
  }, [total, loading]);

  // addedCount 展示后 4 秒自动消失
  useEffect(() => {
    if (addedCount === null) return;
    const timer = setTimeout(() => setAddedCount(null), 4000);
    return () => clearTimeout(timer);
  }, [addedCount]);

  // 页面恢复可见时全量刷新
  useEffect(() => {
    onVisibilityRecover(() => {
      preRefreshTotalRef.current = total;
      isAutoRefreshingRef.current = true;
      fetchLogs();
      setPendingEventCount(0);
    });
  }, [fetchLogs, onVisibilityRecover, total]);

  // ─── 分页 ───

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    // 手动翻回第 1 页时清除待查看横幅
    if (newPage === 1) {
      setPendingEventCount(0);
    }
  };

  return (
    <div>
      <div style={{ marginBottom: 16 }}>
        <Title heading={3} style={{ margin: 0 }}>
          请求日志
        </Title>
      </div>

      <LogFilterBar
        users={users.map((user) => ({ id: user.id, label: user.display_name }))}
        accessPoints={accessPoints.map((accessPoint) => ({
          id: accessPoint.id,
          label: accessPoint.name,
        }))}
        userId={filters.userId}
        accessPointId={filters.accessPointId}
        onDateChange={handleDateChange}
        onUserChange={(userId) => setFilters((prev) => ({ ...prev, userId }))}
        onAccessPointChange={(accessPointId) => setFilters((prev) => ({ ...prev, accessPointId }))}
        onReset={handleReset}
        hideUserSelect
        hideAccessPointSelect
        beforeReset={
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <ConnectionIndicator status={sseStatus} />
            <Button icon={<IconRefresh />} loading={loading} onClick={() => fetchLogs()}>
              刷新
            </Button>
          </div>
        }
      >
        <div>
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>会话 ID</Text>
          <Input
            placeholder="输入会话 ID"
            value={filters.sessionId}
            onChange={(v: string) => setFilters((prev) => ({ ...prev, sessionId: v || undefined }))}
            style={{ width: 180 }}
          />
        </div>
        <div>
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>用户</Text>
          <Select
            placeholder="选择用户"
            value={filters.userId}
            onChange={(v) =>
              setFilters((prev) => ({ ...prev, userId: v == null ? undefined : String(v) }))
            }
            style={{ width: 120 }}
            showClear
          >
            {users.map((u) => (
              <Select.Option key={u.id} value={u.id}>
                {u.display_name}
              </Select.Option>
            ))}
          </Select>
        </div>
        <div>
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>接入点</Text>
          <Select
            placeholder="选择接入点"
            value={filters.accessPointId}
            onChange={(v) =>
              setFilters((prev) => ({ ...prev, accessPointId: v == null ? undefined : String(v) }))
            }
            style={{ width: 120 }}
            showClear
          >
            {accessPoints.map((ap) => (
              <Select.Option key={ap.id} value={ap.id}>
                {ap.name}
              </Select.Option>
            ))}
          </Select>
        </div>
        <div>
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>服务商</Text>
          <Select
            placeholder="选择服务商"
            value={filters.providerId}
            onChange={(v) =>
              setFilters((prev) => ({ ...prev, providerId: v == null ? undefined : String(v) }))
            }
            style={{ width: 120 }}
            showClear
          >
            {providers.map((p) => (
              <Select.Option key={p.id} value={p.id}>
                {p.name}
              </Select.Option>
            ))}
          </Select>
        </div>
        <div>
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>账号</Text>
          <Select
            placeholder="选择账号"
            value={filters.accountId}
            onChange={(v) =>
              setFilters((prev) => ({ ...prev, accountId: v == null ? undefined : String(v) }))
            }
            style={{ width: 120 }}
            showClear
          >
            {accounts.map((a) => (
              <Select.Option key={a.id} value={a.id}>
                {a.name}
              </Select.Option>
            ))}
          </Select>
        </div>
        <div>
          <Text style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>中断</Text>
          <Select
            placeholder="不限"
            value={filters.isInterrupted}
            onChange={(v) =>
              setFilters((prev) => ({
                ...prev,
                isInterrupted: v == null ? undefined : String(v),
              }))
            }
            style={{ width: 80 }}
            showClear
          >
            <Select.Option value="true">是</Select.Option>
            <Select.Option value="false">否</Select.Option>
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
              <Select.Option key={opt.value} value={opt.value}>
                {opt.label}
              </Select.Option>
            ))}
          </Select>
        </div>
      </LogFilterBar>

      {addedCount !== null && (
        <div
          style={{
            background: 'var(--semi-color-success-light-default)',
            padding: '8px 16px',
            borderRadius: 4,
            marginBottom: 12,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <span>新增了 {addedCount} 条日志</span>
          <Button size="small" type="tertiary" onClick={() => setAddedCount(null)}>
            知道了
          </Button>
        </div>
      )}

      {pendingEventCount > 0 && (
        <div
          style={{
            background: 'var(--semi-color-info-light-default)',
            padding: '8px 16px',
            borderRadius: 4,
            marginBottom: 12,
          }}
        >
          <span>有 {pendingEventCount} 条新日志，返回第 1 页查看</span>
        </div>
      )}

      <RequestLogTable
        logs={logs}
        loading={loading}
        total={total}
        page={page}
        pageSize={pageSize}
        userMap={userMap}
        apMap={apMap}
        providerMap={providerMap}
        accountMap={accountMap}
        onPageChange={handlePageChange}
      />
    </div>
  );
}
