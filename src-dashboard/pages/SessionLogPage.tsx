import { type ReactNode, useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { Button, Toast, Typography } from '@douyinfe/semi-ui';
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
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [sessionsLoading, setSessionsLoading] = useState(false);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const [filters, setFilters] = useState<SessionListFilters>({});

  // 详情模式状态
  const [sessionTurns, setSessionTurns] = useState<ConversationTurn[]>([]);
  const [detailLoading, setDetailLoading] = useState(false);
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
      console.warn('[SessionLogPage] 加载会话列表失败');
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
      // 1. 并行加载会话内容和 Token 用量
      const [contents, usage] = await Promise.all([
        api.get<SessionContentItem[]>(`/api/logs/sessions/${encodeURIComponent(sid)}/contents`),
        api.get<TokenUsage[]>(`/api/logs/sessions/${encodeURIComponent(sid)}/token-usage`),
      ]);

      // 2. 构建 Token 用量映射
      const usageMap: Record<string, TokenUsage> = {};
      usage.forEach((tu) => {
        usageMap[tu.log_id] = tu;
      });

      // 3. 使用 buildConversationTurns 构建轮次数据（Token 聚合在内部完成）
      const turns = buildConversationTurns(contents, usageMap);
      setSessionTurns(turns);
    } catch (err) {
      Toast.error(err instanceof Error ? err.message : '加载会话详情失败');
      setSessionTurns([]);
    } finally {
      setDetailLoading(false);
    }
  }, []);

  useEffect(() => {
    if (sessionId) {
      loadSessionDetail(sessionId);
    }
    return () => {
      setSessionTurns([]);
    };
  }, [sessionId, loadSessionDetail]);

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

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
  };

  // ─── 详情视图 ───

  if (sessionId) {
    return (
      <SessionDetailView
        sessionId={sessionId}
        turns={sessionTurns}
        detailLoading={detailLoading}
        onBack={() => navigate('/sessions')}
        onRefresh={() => loadSessionDetail(sessionId)}
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
      <div style={{ marginBottom: 16 }}>
        <Title heading={3} style={{ margin: 0 }}>
          会话日志
        </Title>
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
        beforeReset={
          <Button icon={<IconRefresh />} loading={sessionsLoading} onClick={() => fetchSessions()}>
            刷新
          </Button>
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
