import type { ReactNode } from 'react';
import { Button, Empty, Table, Tag, Tooltip } from '@douyinfe/semi-ui';
import CopyableIdText from '@components/common/CopyableIdText';
import type { LogSummary } from '../../types/log.ts';
import { formatDateTime, formatDuration, truncateMiddle } from '../../utils/format.ts';

interface RequestLogTableProps {
  logs: LogSummary[];
  loading: boolean;
  total: number;
  page: number;
  pageSize: number;
  userMap: Record<string, string>;
  apMap: Record<string, string>;
  onPageChange: (page: number) => void;
}

export default function RequestLogTable({
  logs,
  loading,
  total,
  page,
  pageSize,
  userMap,
  apMap,
  onPageChange,
}: RequestLogTableProps): ReactNode {
  const columns = [
    {
      title: 'ID',
      dataIndex: 'id',
      width: 240,
      render: (id: string) => <CopyableIdText value={id}/>,
    },
    {
      title: '时间',
      dataIndex: 'timestamp',
      width: 180,
      render: (t: string) => formatDateTime(t),
    },
    {
      title: '来源',
      key: 'source',
      width: 80,
      render: (_: unknown, r: LogSummary) => (
        <div style={{display: 'flex', gap: 4, flexWrap: 'wrap'}}>
          <Tag color={r.conversation_source === 'subagent' ? 'green' : 'blue'}>
            {r.conversation_source === 'subagent' ? '子代理' : '主代理'}
          </Tag>
        </div>
      ),
    },
    {
      title: '会话 ID',
      dataIndex: 'session_id',
      width: 260,
      render: (id: string) => <CopyableIdText value={id}/>,
    },
    {
      title: '用户',
      key: 'user',
      width: 80,
      render: (_: unknown, r: LogSummary) =>
        r.user_id ? (userMap[r.user_id] || truncateMiddle(r.user_id, 8)) : '-',
    },
    {
      title: '接入点',
      key: 'ap',
      width: 80,
      render: (_: unknown, r: LogSummary) => {
        if (!r.access_point_id) {
          return '-';
        }

        const accessPointName = apMap[r.access_point_id];
        if (accessPointName) {
          return <span className="nowrap-text">{accessPointName}</span>;
        }

        return <CopyableIdText value={r.access_point_id}/>;
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
      width: 150,
      render: (_: unknown, record: LogSummary) => {
        const hasToken = record.token_input_tokens != null || record.token_output_tokens != null;
        if (!hasToken) return <span style={{color: 'var(--semi-color-text-2)'}}>-</span>;
        const input = record.token_input_tokens?.toLocaleString() || '0';
        const output = record.token_output_tokens?.toLocaleString() || '0';
        const total = record.token_total_tokens?.toLocaleString() || '0';
        return (
          <Tooltip
            content={
              <div style={{fontSize: 12, lineHeight: 1.6}}>
                <div>输入 token：{input}</div>
                <div>输出 token：{output}</div>
                <div>总计：{total}</div>
              </div>
            }
          >
            <span style={{whiteSpace: 'nowrap', cursor: 'default'}}>
              ↑{input} / ↓{output}
            </span>
          </Tooltip>
        );
      },
    },
    {
      title: '耗时',
      dataIndex: 'duration_ms',
      width: 100,
      render: (ms?: number | null) => <span className="nowrap-text">{formatDuration(ms)}</span>,
    },
    {
      title: '操作',
      key: 'actions',
      width: 80,
      render: (_: unknown, record: LogSummary) => (
        <Button
          size="small"
          onClick={() => {
            window.open(`/logs/${record.id}`, '_blank', 'noopener');
          }}
        >
          详情
        </Button>
      ),
    },
  ];

  return (
    <Table
      columns={columns}
      dataSource={logs}
      loading={loading}
      rowKey="id"
      scroll={{x: 'max-content'}}
      pagination={{
        currentPage: page,
        pageSize,
        total,
        onChange: onPageChange,
      }}
      empty={
        <Empty description={loading ? '' : '暂无日志数据'}/>
      }
    />
  );
}
