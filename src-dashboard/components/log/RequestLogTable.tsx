import type { ReactNode } from 'react';
import { Button, Empty, Table, Tag } from '@douyinfe/semi-ui';
import CopyableIdText from '@components/common/CopyableIdText';
import type { LogSummary } from '../../types/log.ts';
import { formatDateTime, formatDuration, truncateMiddle } from '../../utils/format.ts';
import TokenCell from './TokenCell.tsx';

/** RequestLogTable 组件 Props */
interface RequestLogTableProps {
  logs: LogSummary[];
  loading: boolean;
  total: number;
  page: number;
  pageSize: number;
  userMap: Record<string, string>;
  apMap: Record<string, string>;
  providerMap: Record<string, string>;
  accountMap: Record<string, string>;
  onPageChange: (page: number) => void;
}

/**
 * RequestLogTable - 请求日志表格组件
 *
 * 展示日志列表，包含 ID、时间、来源、会话、用户、模型、状态码、Token 耗时等列。
 */
export default function RequestLogTable({
  logs,
  loading,
  total,
  page,
  pageSize,
  userMap,
  apMap,
  providerMap,
  accountMap,
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
      title: '服务商',
      key: 'provider',
      width: 100,
      render: (_: unknown, r: LogSummary) => {
        if (!r.provider_id) {
          return '-';
        }

        const providerName = providerMap[r.provider_id];
        if (providerName) {
          return <span className="nowrap-text">{providerName}</span>;
        }

        return <CopyableIdText value={r.provider_id}/>;
      },
    },
    {
      title: '账号',
      key: 'account',
      width: 100,
      render: (_: unknown, r: LogSummary) => {
        if (!r.account_id) {
          return '-';
        }

        const accountName = accountMap[r.account_id];
        if (accountName) {
          return <span className="nowrap-text">{accountName}</span>;
        }

        return <CopyableIdText value={r.account_id}/>;
      },
    },
    {
      title: '中断',
      dataIndex: 'is_interrupted',
      width: 60,
      render: (v: boolean) => (
        <Tag color={v ? 'red' : undefined}>{v ? '是' : '否'}</Tag>
      ),
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
      render: (_: unknown, record: LogSummary) => (
        <TokenCell
          input_tokens={record.token_input_tokens}
          output_tokens={record.token_output_tokens}
          cache_creation_input_tokens={record.token_cache_creation_input_tokens}
          cache_read_input_tokens={record.token_cache_read_input_tokens}
          thinking_tokens={record.token_thinking_tokens}
          total_tokens={record.token_total_tokens}
        />
      ),
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
