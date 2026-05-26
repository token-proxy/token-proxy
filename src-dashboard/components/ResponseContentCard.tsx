import { useState, useMemo, type ReactNode } from 'react';
import { Card, Collapse, Switch, Typography } from '@douyinfe/semi-ui';
import MarkdownRender from './MarkdownRender.tsx';
import RawResponseView from './RawResponseView.tsx';
import { extractAssistantTextFromSSE, extractThinkingFromSSE } from '../utils/parseLogs.ts';

const { Text } = Typography;

interface ResponseContentCardProps {
  responseBody: string | null | undefined;
  style?: React.CSSProperties;
}

export default function ResponseContentCard({
  responseBody,
  style,
}: ResponseContentCardProps): ReactNode {
  const [viewMode, setViewMode] = useState<'formatted' | 'json'>('formatted');

  const hasBody = !!responseBody;

  const assistantText = useMemo(
    () => extractAssistantTextFromSSE(responseBody ?? ''),
    [responseBody],
  );
  const thinkingText = useMemo(
    () => extractThinkingFromSSE(responseBody ?? ''),
    [responseBody],
  );

  const hasAssistant = !!assistantText;
  const hasThinking = !!thinkingText;

  return (
    <Card
      title={
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%' }}>
          <span>响应内容</span>
          {hasBody && (
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <Text size="small" style={{ color: 'var(--semi-color-text-2)' }}>
                {viewMode === 'formatted' ? '格式化' : '原始 SSE'}
              </Text>
              <Switch
                size="small"
                checked={viewMode === 'json'}
                onChange={(checked) => setViewMode(checked ? 'json' : 'formatted')}
              />
            </div>
          )}
        </div>
      }
      bodyStyle={{ padding: '20px 24px' }}
      style={style}
    >
      {viewMode === 'formatted' ? (
        <>
          {hasThinking ? (
            <Collapse defaultActiveKey={[]} style={{ marginBottom: 16 }}>
              <Collapse.Panel header="思考内容" itemKey="thinking">
                <pre
                  style={{
                    background: 'var(--semi-color-fill-0)',
                    padding: 12,
                    borderRadius: 4,
                    fontSize: 12,
                    whiteSpace: 'pre-wrap',
                    wordBreak: 'break-all',
                    margin: 0,
                  }}
                >
                  {thinkingText}
                </pre>
              </Collapse.Panel>
            </Collapse>
          ) : (
            !hasAssistant && (
              <Text type="secondary" style={{ display: 'block', marginBottom: 16 }}>
                (无响应内容)
              </Text>
            )
          )}

          {hasAssistant ? (
            <div>
              <Text strong style={{ display: 'block', marginBottom: 8 }}>
                助手回复:
              </Text>
              <MarkdownRender content={assistantText!} />
            </div>
          ) : (
            hasThinking && <Text type="secondary">(无助手回复)</Text>
          )}
        </>
      ) : (
        <RawResponseView body={responseBody!} />
      )}
    </Card>
  );
}
