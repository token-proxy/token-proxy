import type { ReactNode } from 'react';
import {
  Table, Button, Typography, Empty, Tooltip,
} from '@douyinfe/semi-ui';
import type { DatePickerProps } from '@douyinfe/semi-ui/lib/es/datePicker';
import CopyableIdText from './CopyableIdText.tsx';
import LogFilterBar from './LogFilterBar.tsx';
import type {
  AccessPointItem,
  SessionListFilters,
  SessionSummary,
  UserItem,
} from '../types/log.ts';
import { formatDateTime } from '../utils/format.ts';

const { Text } = Typography;

interface SessionListViewProps {
  users: UserItem[];
  accessPoints: AccessPointItem[];
  userMap: Record<string, string>;
  apMap: Record<string, string>;
  sessions: SessionSummary[];
  loading: boolean;
  total: number;
  page: number;
  pageSize: number;
  filters: SessionListFilters;
  onDateChange: DatePickerProps['onChange'];
  onUserChange: (userId: string | undefined) => void;
  onAccessPointChange: (accessPointId: string | undefined) => void;
  onReset: () => void;
  onPageChange: (page: number) => void;
}

export default function SessionListView({
  users,
  accessPoints,
  userMap,
  apMap,
  sessions,
  loading,
  total,
  page,
  pageSize,
  filters,
  onDateChange,
  onUserChange,
  onAccessPointChange,
  onReset,
  onPageChange,
}: SessionListViewProps): ReactNode {
  return (
    <div>
      <LogFilterBar
        users={users.map((user) => ({ id: user.id, label: user.display_name }))}
        accessPoints={accessPoints.map((accessPoint) => ({ id: accessPoint.id, label: accessPoint.name }))}
        userId={filters.userId}
        accessPointId={filters.accessPointId}
        datePickerWidth={360}
        selectWidth={160}
        onDateChange={onDateChange}
        onUserChange={onUserChange}
        onAccessPointChange={onAccessPointChange}
        onReset={onReset}
      />

      <Table
        columns={[
          {
            title: '会话 ID',
            dataIndex: 'session_id',
            key: 'session_id',
            width: 280,
            render: (id: string) => (
              <CopyableIdText value={id} />
            ),
          },
          {
            title: '用户',
            key: 'user',
            width: 100,
            render: (_: unknown, r: SessionSummary) => {
              if (!r.user_id) return '-';
              const name = userMap[r.user_id];
              return name
                ? <span style={{ whiteSpace: 'nowrap' }}>{name}</span>
                : <CopyableIdText value={r.user_id} />;
            },
          },
          {
            title: '接入点',
            key: 'ap',
            width: 80,
            render: (_: unknown, r: SessionSummary) => {
              if (!r.access_point_id) return '-';
              const name = apMap[r.access_point_id];
              return name
                ? <Text ellipsis style={{ maxWidth: 110 }}>{name}</Text>
                : <CopyableIdText value={r.access_point_id} />;
            },
          },
          {
            title: '开始时间',
            dataIndex: 'start_time',
            width: 180,
            render: (t: string) => <span style={{ whiteSpace: 'nowrap' }}>{formatDateTime(t)}</span>,
          },
          {
            title: '请求次数',
            dataIndex: 'request_count',
            width: 100,
            render: (v: number) => (
              <span style={{ whiteSpace: 'nowrap', display: 'block', maxWidth: 90 }}>{v}</span>
            ),
          },
          {
            title: 'Token',
            key: 'token',
            width: 150,
            render: (_: unknown, record: SessionSummary) => {
              const hasToken = record.total_input_tokens > 0 || record.total_output_tokens > 0;
              if (!hasToken) return <span style={{ color: 'var(--semi-color-text-2)' }}>-</span>;
              return (
                <Tooltip
                  content={
                    <div style={{ fontSize: 12, lineHeight: 1.6 }}>
                      <div>总输入: {record.total_input_tokens.toLocaleString()}</div>
                      <div>总输出: {record.total_output_tokens.toLocaleString()}</div>
                      <div>缓存创建: {record.total_cache_creation_input_tokens.toLocaleString()}</div>
                      <div>缓存读取: {record.total_cache_read_input_tokens.toLocaleString()}</div>
                      <div>思考: {record.total_thinking_tokens.toLocaleString()}</div>
                      <div>总计: {record.total_tokens.toLocaleString()}</div>
                    </div>
                  }
                >
                  <span style={{ whiteSpace: 'nowrap', cursor: 'default' }}>
                    &uarr;{record.total_input_tokens.toLocaleString()} / &darr;{record.total_output_tokens.toLocaleString()}
                  </span>
                </Tooltip>
              );
            },
          },
          {
            title: '操作',
            key: 'actions',
            width: 100,
              render: (_: unknown, r: SessionSummary) => (
              <Button
                size="small"
                onClick={() => {
                  window.open(
                    `/sessions/${encodeURIComponent(r.session_id)}`,
                    '_blank',
                    'noopener',
                  );
                }}
              >
                查看详情
              </Button>
            ),
          },
        ]}
        dataSource={sessions}
        loading={loading}
        rowKey="session_id"
        scroll={{ x: 'max-content' }}
        pagination={{
          currentPage: page,
          pageSize,
          total,
          onChange: onPageChange,
        }}
        empty={
          <Empty description={loading ? '' : '暂无会话数据'} />
        }
      />
    </div>
  );
}
