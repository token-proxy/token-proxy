import { type ReactNode } from 'react';
import { Card, Descriptions, Tag } from '@douyinfe/semi-ui';
import type { LogDetailFull } from '../../../types/log.ts';
import { formatDateTime, formatNumber } from '../../../utils/format.ts';
import { CLIENT_TYPE_LABELS } from '../../../utils/clientType.ts';
import CopyableIdText from '@components/common/CopyableIdText';

const SOURCE_LABELS: Record<string, string> = {
  main: '主代理',
  subagent: '子代理',
  unknown: '未知',
};

const SOURCE_COLORS = {
  main: 'blue',
  subagent: 'green',
  unknown: 'grey',
} as const;

/** BasicInfoCard 组件 Props */
interface BasicInfoCardProps {
  data: LogDetailFull;
  style?: React.CSSProperties;
}

/**
 * BasicInfoCard - 日志基础信息卡片
 *
 * 展示请求 ID、时间、会话、用户、接入点、模型映射、状态码、耗时等关键信息。
 */
export default function BasicInfoCard({ data: d, style }: BasicInfoCardProps): ReactNode {
  const items: Array<{ key: string; value: ReactNode }> = [
    { key: '请求 ID', value: <CopyableIdText value={d.id} /> },
    { key: '时间', value: formatDateTime(d.timestamp) },
    { key: '会话 ID', value: <CopyableIdText value={d.session_id} /> },
  ];

  if (d.user_id) {
    items.push({
      key: '用户',
      value: d.user_name ? <span>{d.user_name}</span> : <CopyableIdText value={d.user_id} />,
    });
  }

  if (d.access_point_id) {
    items.push({
      key: '接入点',
      value: d.access_point_name ? (
        <span>{d.access_point_name}</span>
      ) : (
        <CopyableIdText value={d.access_point_id} />
      ),
    });
  }

  items.push({
    key: '模型映射',
    value: (
      <span>
        <Tag color="blue" size="small">
          {d.model_original || '-'}
        </Tag>
        {' → '}
        <Tag color="blue" size="small">
          {d.model_mapped || '-'}
        </Tag>
      </span>
    ),
  });

  items.push({
    key: '状态码',
    value: (
      <Tag color={(d.status_code ?? 0) >= 400 ? 'red' : 'green'} size="small">
        {d.status_code ?? '-'}
      </Tag>
    ),
  });

  items.push({
    key: '耗时',
    value: `${formatNumber(d.duration_ms, false)} ms`,
  });

  items.push({
    key: '来源',
    value: (
      <Tag
        color={SOURCE_COLORS[d.conversation_source as keyof typeof SOURCE_COLORS] || 'grey'}
        size="small"
      >
        {SOURCE_LABELS[d.conversation_source] || d.conversation_source}
      </Tag>
    ),
  });

  if (d.agent_id) {
    items.push({
      key: 'Agent ID',
      value: <CopyableIdText value={d.agent_id} />,
    });
  }

  // 客户端信息：优先展示 client_type 中文名 + 版本号，辅助展示原始 UA
  const clientTypeLabel = CLIENT_TYPE_LABELS[d.client_type || ''] || d.client_type;
  const clientDisplay = [clientTypeLabel, d.client_version].filter(Boolean).join(' ');
  if (clientDisplay) {
    items.push({
      key: '客户端',
      value: (
        <div>
          <span>{clientDisplay}</span>
          {d.client_user_agent && (
            <div
              className="dashboard-deleted"
              style={{ fontSize: 11, marginTop: 2, wordBreak: 'break-all' }}
            >
              {d.client_user_agent}
            </div>
          )}
        </div>
      ),
    });
  }

  return (
    <Card title="基础信息" style={style} bodyStyle={{ padding: '20px 24px' }}>
      <Descriptions data={items} row size="small" />
    </Card>
  );
}
