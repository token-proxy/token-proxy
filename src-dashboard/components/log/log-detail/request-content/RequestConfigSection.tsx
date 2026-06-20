import { type ReactNode } from 'react';
import { Descriptions } from '@douyinfe/semi-ui';
import AccordionSection from './AccordionSection';

interface RequestConfigSectionProps {
  requestBody: Record<string, unknown>;
}

/**
 * RequestConfigSection - 请求配置展示区块
 *
 * 展示请求中的模型、最大 Token、流式、思考模式、输出配置等参数。
 */
export default function RequestConfigSection({
  requestBody,
}: RequestConfigSectionProps): ReactNode {
  const items: Array<{ key: string; value: string }> = [];

  if (typeof requestBody.model === 'string') {
    items.push({key: '模型', value: requestBody.model});
  }
  if (typeof requestBody.max_tokens === 'number') {
    items.push({key: '最大 Token 数', value: String(requestBody.max_tokens)});
  }
  if (requestBody.stream !== undefined) {
    items.push({key: '流式', value: requestBody.stream ? '是' : '否'});
  }

  const thinking = requestBody.thinking as
    | { type?: string; budget_tokens?: number }
    | undefined;
  if (thinking?.type) {
    const parts: string[] = [thinking.type];
    if (typeof thinking.budget_tokens === 'number') {
      parts.push(`预算: ${thinking.budget_tokens}`);
    }
    items.push({key: '思考模式', value: parts.join(', ')});
  }

  const outputConfig = requestBody.output_config as
    | { effort?: string }
    | undefined;
  if (outputConfig?.effort) {
    items.push({key: '输出配置', value: `effort: ${outputConfig.effort}`});
  }

  if (items.length === 0) return null;

  return (
    <AccordionSection title="请求配置" defaultExpanded={false}>
      <Descriptions
        row
        size="small"
        data={items.map((d) => ({key: d.key, value: d.value}))}
      />
    </AccordionSection>
  );
}
