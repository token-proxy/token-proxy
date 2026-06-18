import { type ReactNode } from 'react';
import { Card, Descriptions } from '@douyinfe/semi-ui';
import type { LogDetailFull } from '../../../types/log.ts';
import { formatNumber } from '../../../utils/format.ts';

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

export default function TokenUsageCard({data: d, style}: TokenUsageCardProps): ReactNode {
  if (!hasTokenData(d)) return null;

  const total = d.token_total_tokens ?? 0;
  const input = d.token_input_tokens ?? 0;
  const output = d.token_output_tokens ?? 0;
  const cacheCreate = d.token_cache_creation_input_tokens ?? 0;
  const cacheRead = d.token_cache_read_input_tokens ?? 0;
  const thinking = d.token_thinking_tokens ?? 0;

  return (
    <Card
      title="Token 用量"
      style={style}
      bodyStyle={{padding: '20px 24px'}}
    >
      <Descriptions row size="small">
        <Descriptions.Item itemKey={<strong>总计</strong>}>{formatNumber(total)}</Descriptions.Item>
        <Descriptions.Item itemKey="新输入">{formatNumber(input)}</Descriptions.Item>
        <Descriptions.Item itemKey="缓存读取">{formatNumber(cacheRead)}</Descriptions.Item>
        <Descriptions.Item itemKey="缓存创建">{formatNumber(cacheCreate)}</Descriptions.Item>
        <Descriptions.Item itemKey="输出">{formatNumber(output)}</Descriptions.Item>
        <Descriptions.Item itemKey="思考">{formatNumber(thinking)}</Descriptions.Item>
      </Descriptions>
    </Card>
  );
}
