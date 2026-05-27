import { useState, useEffect, useCallback, useMemo, type ReactNode } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Button, Typography, Toast,
} from '@douyinfe/semi-ui';
import { IconRefresh } from '@douyinfe/semi-icons';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import api from '../api.ts';
import SessionListView from '../components/SessionListView.tsx';
import SessionDetailView from '../components/SessionDetailView.tsx';
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
import { buildConversationEvents } from '../utils/parseLogs.ts';
import { buildQueryString, toIsoString } from '../utils/query.ts';

const { Title } = Typography;

// ─── 组件 ───

export default function SessionLogPage(): ReactNode {
  const { sessionId } = useParams<{ sessionId: string }>();
  const navigate = useNavigate();

  // 参考数据，用于查找映射
  const [users, setUsers] = useState<UserItem[]>([]);
  const [accessPoints, setAccessPoints] = useState<AccessPointItem[]>([]);

  // 列表模式状态
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [sessionsLoading, setSessionsLoading] = useState(false);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const [filters, setFilters] = useState<SessionListFilters>({});

  // 详情模式状态
  const [sessionEvents, setSessionEvents] = useState<ConversationEvent[]>([]);
  const [sessionTokenUsage, setSessionTokenUsage] = useState<TokenUsage[]>([]);
  const [detailLoading, setDetailLoading] = useState(false);
  const [rawModalVisible, setRawModalVisible] = useState(false);
  const [rawModalTitle, setRawModalTitle] = useState('');
  const [rawModalContent, setRawModalContent] = useState('');

  // ─── 加载参考数据 ───

  useEffect(() => {
    api.get<UserItem[]>('/api/users')
      .then(setUsers)
      .catch(() => {});
    api.get<AccessPointItem[]>('/api/access-points')
      .then(setAccessPoints)
      .catch(() => {});
  }, []);

  // ─── 查找映射 ───

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

  // ─── 加载会话（列表模式） ───

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

  // ─── 加载会话详情 ───

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

  // ─── 弹窗辅助 ───

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

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
  };

  // ─── Token 用量映射 ───

  const tokenUsageMap = useMemo(() => {
    const m: Record<string, TokenUsage> = {};
    sessionTokenUsage.forEach((tu) => { m[tu.log_id] = tu; });
    return m;
  }, [sessionTokenUsage]);

  // ─── 详情视图 ───

  if (sessionId) {
    const sortedEvents = [...sessionEvents].sort((a, b) => {
      if (a.request_index !== b.request_index) return a.request_index - b.request_index;
      if (a.event_index !== b.event_index) return a.event_index - b.event_index;
      const timeCmp = new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime();
      if (timeCmp !== 0) return timeCmp;
      return a.id.localeCompare(b.id);
    });

    return (
      <SessionDetailView
        sessionId={sessionId}
        sortedEvents={sortedEvents}
        tokenUsageMap={tokenUsageMap}
        detailLoading={detailLoading}
        onBack={() => navigate('/sessions')}
        onRefresh={() => {
          loadSessionDetail(sessionId);
          fetchSessionTokenUsage(sessionId);
        }}
        onOpenRaw={openRawModal}
        rawModalVisible={rawModalVisible}
        rawModalTitle={rawModalTitle}
        rawModalContent={rawModalContent}
        onCloseRawModal={closeRawModal}
      />
    );
  }

  // ─── 列表视图 ───

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
        onDateChange={handleDateChange}
        onUserChange={(userId) => setFilters((prev) => ({ ...prev, userId }))}
        onAccessPointChange={(accessPointId) =>
          setFilters((prev) => ({ ...prev, accessPointId }))
        }
        onReset={handleResetFilters}
        onPageChange={handlePageChange}
      />
    </div>
  );
}
