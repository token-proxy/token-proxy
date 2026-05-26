import { type ReactNode } from 'react';
import { Card, Descriptions } from '@douyinfe/semi-ui';
import type { LogDetailFull } from '../types/log.ts';
import { formatNumber } from '../utils/format.ts';

interface TokenUsageCardProps {
  data: LogDetailFull;
  style?: React.CSSProperties;
}

export function hasTokenData(d: LogDetailFull): boolean {
  return (
    d.token_input_tokens != null ||
    d.token_output_tokens != null ||
    d.token_cache_creation_input_tokens != null ||
    d.token_cache_read_input_tokens != null ||
    d.token_thinking_tokens != null ||
    d.token_total_tokens != null
  );
}

export default function TokenUsageCard({ data: d, style }: TokenUsageCardProps): ReactNode {
  if (!hasTokenData(d)) return null;

  const items = [
    { key: '输入 Tokens', value: formatNumber(d.token_input_tokens ?? 0) },
    { key: '输出 Tokens', value: formatNumber(d.token_output_tokens ?? 0) },
    { key: '缓存创建', value: formatNumber(d.token_cache_creation_input_tokens ?? 0) },
    { key: '缓存读取', value: formatNumber(d.token_cache_read_input_tokens ?? 0) },
    { key: '思考 Tokens', value: formatNumber(d.token_thinking_tokens ?? 0) },
    { key: '总计', value: formatNumber(d.token_total_tokens ?? 0) },
  ];

  return (
    <Card
      title="Token 用量"
      style={style}
      bodyStyle={{ padding: '20px 24px' }}
    >
      <Descriptions data={items} row size="small" />
    </Card>
  );
}
