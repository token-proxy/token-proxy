import { useState, useMemo, type ReactNode } from 'react';
import { Card, Switch, Typography } from '@douyinfe/semi-ui';
import MarkdownRender from './MarkdownRender.tsx';
import CodeHighlight from './CodeHighlight.tsx';
import { extractLastUserMessage } from '../utils/parseLogs.ts';

const { Text } = Typography;

interface RequestContentCardProps {
  requestBody: Record<string, unknown> | null | undefined;
  style?: React.CSSProperties;
}

export default function RequestContentCard({
  requestBody,
  style,
}: RequestContentCardProps): ReactNode {
  const [viewMode, setViewMode] = useState<'formatted' | 'json'>('formatted');

  const hasContent = requestBody != null && Object.keys(requestBody).length > 0;
  const jsonText = useMemo(
    () => (requestBody != null ? JSON.stringify(requestBody, null, 2) : ''),
    [requestBody],
  );
  const messageText = useMemo(
    () => extractLastUserMessage(requestBody),
    [requestBody],
  );
  const hasMessage = !!messageText;

  return (
    <Card
      title={
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%' }}>
          <span>请求内容</span>
          {hasContent && (
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <Text size="small" style={{ color: 'var(--semi-color-text-2)' }}>
                {viewMode === 'formatted' ? '格式化' : 'JSON'}
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
      style={style}
      bodyStyle={{ padding: '20px 24px' }}
    >
      {hasContent ? (
        viewMode === 'formatted' && hasMessage ? (
          <MarkdownRender content={messageText!} />
        ) : (
          <CodeHighlight content={jsonText} />
        )
      ) : (
        <Text type="secondary">(无请求内容)</Text>
      )}
    </Card>
  );
}
