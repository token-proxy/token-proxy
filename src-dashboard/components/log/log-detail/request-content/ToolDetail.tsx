import { type ReactNode } from 'react';
import { Descriptions, Typography } from '@douyinfe/semi-ui';
import MarkdownRender from '@components/common/MarkdownRender';

const {Text} = Typography;

interface ToolDetailProps {
  tool: Record<string, unknown>;
}

export default function ToolDetail({tool}: ToolDetailProps): ReactNode {
  const description = typeof tool.description === 'string' ? tool.description : '';
  const inputSchema = tool.input_schema as
    | { properties?: Record<string, unknown>; required?: Array<string> }
    | undefined;
  const props = inputSchema?.properties;
  const required = inputSchema?.required ?? [];

  return (
    <div
      style={{
        padding: 12,
        border: '1px solid var(--semi-color-border)',
        borderRadius: 6,
      }}
    >
      {description && (
        <div style={{marginBottom: 12}}>
          <Text size="small" style={{color: 'var(--semi-color-text-2)', marginBottom: 4, display: 'block'}}>
            描述
          </Text>
          <MarkdownRender content={description}/>
        </div>
      )}
      {props && Object.keys(props).length > 0 && (
        <div>
          <Text size="small" style={{color: 'var(--semi-color-text-2)', marginBottom: 8, display: 'block'}}>
            参数
          </Text>
          <Descriptions
            row
            size="small"
            data={Object.entries(props).map(([k, v]) => {
              const propVal = v as { type?: string; description?: string };
              const requiredMark = required.includes(k) ? ' *' : '';
              let valueStr = `类型: ${propVal.type ?? '-'}${requiredMark}`;
              if (propVal.description) {
                valueStr += `, ${propVal.description}`;
              }
              return {key: k, value: valueStr};
            })}
          />
        </div>
      )}
    </div>
  );
}
