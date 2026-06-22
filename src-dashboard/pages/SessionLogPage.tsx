import { type ReactNode, useEffect, useMemo, useState } from 'react';
import { useFetch } from '../hooks/useFetch.ts';
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
        onRefresh={loadSessionDetail}
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
          <Button icon={<IconRefresh />} loading={sessionsLoading} onClick={fetchSessions}>
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
