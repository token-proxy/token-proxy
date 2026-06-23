import { type ReactNode, useEffect, useMemo, useRef, useState } from 'react';
import { useFetch } from '../hooks/useFetch.ts';
import { useLogEvents } from '../hooks/useLogEvents';
import ConnectionIndicator from '@components/common/ConnectionIndicator';
import { useNavigate, useParams } from 'react-router-dom';
import { Button, Typography } from '@douyinfe/semi-ui';
import { IconRefresh } from '@douyinfe/semi-icons';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import api from '../api.ts';
import SessionListView from '@components/session/SessionListView';
import SessionDetailView from '@components/session/SessionDetailView';
import type {
  AccessPointItem,
  ConversationTurn,
  LogDetail,
  PaginatedResult,
  SessionContentItem,
  SessionListFilters,
  SessionSummary,
  TokenUsage,
  UserItem,
} from '../types/log.ts';
import { buildConversationTurns } from '../utils/parseLogs.ts';
import { buildQueryString, toIsoString } from '../utils/query.ts';

const { Title } = Typography;

// ─── 组件 ───

/**
 * SessionLogPage - 会话日志页面
 *
 * 支持列表/详情两种模式：
 * - 无 sessionId 参数时展示会话列表，支持筛选查询
 * - 有 sessionId 参数时展示会话详情，包含对话时间线、Token 用量、事件摘要
 */
export default function SessionLogPage(): ReactNode {
  const { sessionId } = useParams<{ sessionId: string }>();
  const navigate = useNavigate();

  // 参考数据，用于查找映射
  const [users, setUsers] = useState<UserItem[]>([]);
  const [accessPoints, setAccessPoints] = useState<AccessPointItem[]>([]);

  // 列表模式状态
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const [filters, setFilters] = useState<SessionListFilters>({});

  // 详情模式状态
  const [rawModalVisible, setRawModalVisible] = useState(false);
  const [rawModalTitle, setRawModalTitle] = useState('');
  const [rawModalContent, setRawModalContent] = useState('');

  // ─── 加载参考数据 ───

  useEffect(() => {
    api
      .get<UserItem[]>('/api/users')
      .then(setUsers)
      .catch(() => console.warn('[SessionLogPage] 加载用户参考数据失败'));
    api
      .get<AccessPointItem[]>('/api/access-points')
      .then(setAccessPoints)
      .catch(() => console.warn('[SessionLogPage] 加载接入点参考数据失败'));
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

  // ─── 加载会话（列表模式） ───

  const {
    data: sessionsData,
    loading: sessionsLoading,
    refetch: fetchSessions,
  } = useFetch(async () => {
    if (sessionId)
      return {
        items: [],
        total: 0,
        page: 1,
        page_size: pageSize,
      } satisfies PaginatedResult<SessionSummary>;
    const qs = buildQueryString({
      page,
      page_size: pageSize,
      start_time: filters.startTime,
      end_time: filters.endTime,
      user_id: filters.userId,
      access_point_id: filters.accessPointId,
    });
    return api.get<PaginatedResult<SessionSummary>>(`/api/logs/sessions?${qs}`);
  }, [sessionId, page, pageSize, filters]);
  const sessions = sessionsData?.items ?? [];
  const total = sessionsData?.total ?? 0;

  // ─── 加载会话详情 ───

  const {
    data: sessionTurnsData,
    loading: detailLoading,
    refetch: loadSessionDetail,
  } = useFetch(async () => {
    if (!sessionId) return [] as ConversationTurn[];
    // 1. 并行加载会话内容和 Token 用量
    const [contents, usage] = await Promise.all([
      api.get<SessionContentItem[]>(`/api/logs/sessions/${encodeURIComponent(sessionId)}/contents`),
      api.get<TokenUsage[]>(`/api/logs/sessions/${encodeURIComponent(sessionId)}/token-usage`),
    ]);

    // 2. 构建 Token 用量映射
    const usageMap: Record<string, TokenUsage> = {};
    usage.forEach((tu) => {
      usageMap[tu.log_id] = tu;
    });

    // 3. 使用 buildConversationTurns 构建轮次数据（Token 聚合在内部完成）
    return buildConversationTurns(contents, usageMap);
  }, [sessionId]);
  const sessionTurns = sessionTurnsData ?? [];

  // ─── 弹窗辅助 ───

  const openRawModal = async (logId: string) => {
    setRawModalTitle('原始日志内容');
    setRawModalContent('加载中...');
    setRawModalVisible(true);
    try {
      const detail = await api.get<LogDetail>(`/api/logs/${logId}/raw`);
      setRawModalContent(
        [
          '=== 请求头 ===',
          JSON.stringify(detail.request_headers, null, 2),
          '',
          '=== 请求体 ===',
          JSON.stringify(detail.request_body, null, 2),
          '',
          '=== 响应体 ===',
          detail.response_body || '(空)',
        ].join('\n'),
      );
    } catch (err) {
      setRawModalContent(err instanceof Error ? err.message : '加载原始内容失败');
    }
  };

  const closeRawModal = () => {
    setRawModalVisible(false);
  };

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

  const handleResetFilters = () => {
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

  // 判断是否在首页且无筛选（仅在列表模式下生效）
  const isListFirstPageNoFilters = useMemo(
    () => !sessionId && page === 1 && Object.keys(filters).length === 0,
    [sessionId, page, filters],
  );

  // 收到新日志事件时的行为
  useEffect(() => {
    if (!lastEvent) return;

    // 详情模式：匹配 session_id 时增量刷新
    if (sessionId) {
      if (lastEvent.session_id === sessionId) {
        loadSessionDetail();
      }
      return;
    }

    // 列表模式：首页无筛选自动刷新，否则累积计数
    if (isListFirstPageNoFilters) {
      preRefreshTotalRef.current = total;
      isAutoRefreshingRef.current = true;
      fetchSessions();
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
    if (sessionsLoading) return;
    const delta = total - preRefreshTotalRef.current;
    if (delta > 0) {
      setAddedCount(delta);
    }
    isAutoRefreshingRef.current = false;
  }, [total, sessionsLoading]);

  // addedCount 展示后 4 秒自动消失
  useEffect(() => {
    if (addedCount === null) return;
    const timer = setTimeout(() => setAddedCount(null), 4000);
    return () => clearTimeout(timer);
  }, [addedCount]);

  // 页面恢复可见时全量刷新
  useEffect(() => {
    onVisibilityRecover(() => {
      if (sessionId) {
        loadSessionDetail();
      } else {
        preRefreshTotalRef.current = total;
        isAutoRefreshingRef.current = true;
        fetchSessions();
      }
      setPendingEventCount(0);
    });
  }, [sessionId, fetchSessions, loadSessionDetail, onVisibilityRecover, total]);

  // ─── 分页 ───

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    // 手动翻回第 1 页时清除待查看横幅
    if (newPage === 1) {
      setPendingEventCount(0);
    }
  };

  // ─── 详情视图 ───

  if (sessionId) {
    return (
      <SessionDetailView
        sessionId={sessionId}
        turns={sessionTurns}
        detailLoading={detailLoading}
        onBack={() => navigate('/sessions')}
        onRefresh={loadSessionDetail}
        onOpenRaw={openRawModal}
        rawModalVisible={rawModalVisible}
        rawModalTitle={rawModalTitle}
        rawModalContent={rawModalContent}
        onCloseRawModal={closeRawModal}
        beforeRefresh={<ConnectionIndicator status={sseStatus} />}
      />
    );
  }

  // ─── 列表视图 ───

  return (
    <div>
      <div style={{ marginBottom: 16 }}>
        <Title heading={3} style={{ margin: 0 }}>
          会话日志
        </Title>
      </div>

      {addedCount !== null && !sessionId && (
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
          <span>新增了 {addedCount} 个会话</span>
          <Button size="small" type="tertiary" onClick={() => setAddedCount(null)}>
            知道了
          </Button>
        </div>
      )}

      {pendingEventCount > 0 && !sessionId && (
        <div
          style={{
            background: 'var(--semi-color-info-light-default)',
            padding: '8px 16px',
            borderRadius: 4,
            marginBottom: 12,
          }}
        >
          <span>有 {pendingEventCount} 个新会话，返回第 1 页查看</span>
        </div>
      )}

      <SessionListView
        users={users}
        accessPoints={accessPoints}
        userMap={userMap}
        apMap={apMap}
        sessions={sessions}
        loading={sessionsLoading}
        total={total}
        page={page}
        pageSize={pageSize}
        filters={filters}
        beforeReset={
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <ConnectionIndicator status={sseStatus} />
            <Button icon={<IconRefresh />} loading={sessionsLoading} onClick={fetchSessions}>
              刷新
            </Button>
          </div>
        }
        onDateChange={handleDateChange}
        onUserChange={(userId) => setFilters((prev) => ({ ...prev, userId }))}
        onAccessPointChange={(accessPointId) => setFilters((prev) => ({ ...prev, accessPointId }))}
        onReset={handleResetFilters}
        onPageChange={handlePageChange}
      />
    </div>
  );
}
