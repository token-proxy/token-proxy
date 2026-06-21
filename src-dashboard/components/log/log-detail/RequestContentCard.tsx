import { type ReactNode, useMemo, useState } from 'react';
import { Card, Switch, Typography } from '@douyinfe/semi-ui';
import CodeHighlight from '@components/common/CodeHighlight';
import CollapsibleCard from '@components/common/CollapsibleCard';
import RequestConfigSection from './request-content/RequestConfigSection';
import SystemPromptSection from './request-content/SystemPromptSection';
import MessagesSection from './request-content/MessagesSection';
import ToolsSection from './request-content/ToolsSection';
import './RequestContentCard.css';

const { Text } = Typography;

/** RequestContentCard 组件 Props */
interface RequestContentCardProps {
  requestBody: Record<string, unknown> | null | undefined;
  style?: React.CSSProperties;
}

/**
 * RequestContentCard - 请求内容展示卡片
 *
 * 支持结构化视图（分段展示模型配置、系统提示词、消息、工具）和原始 JSON 视图，
 * 通过 Switch 切换模式。
 */
export default function RequestContentCard({
  requestBody,
  style,
}: RequestContentCardProps): ReactNode {
  const [isJsonViewMode, setIsJsonViewMode] = useState(false);

  const hasContent = requestBody != null && Object.keys(requestBody).length > 0;

  // 原始 JSON 视图：用 useMemo 缓存 JSX 元素树
  const jsonView = useMemo(() => {
    if (!hasContent) return null;
    const jsonText = JSON.stringify(requestBody, null, 2);
    return <CodeHighlight content={jsonText} />;
  }, [requestBody, hasContent]);

  // 结构化视图：用 useMemo 缓存 JSX 元素树
  const formattedView = useMemo(() => {
    if (!hasContent || !requestBody) return null;

    const system = requestBody.system as Array<Record<string, unknown>> | undefined;
    const messages = requestBody.messages as Array<Record<string, unknown>> | undefined;
    const tools = requestBody.tools as Array<Record<string, unknown>> | undefined;

    const hasAnySection = !!(
      system?.length ||
      messages?.length ||
      tools?.length ||
      requestBody.model ||
      requestBody.max_tokens != null ||
      requestBody.stream !== undefined ||
      requestBody.thinking ||
      requestBody.output_config
    );

    if (!hasAnySection) return null;

    return (
      <div>
        <RequestConfigSection requestBody={requestBody} />
        <SystemPromptSection system={system} />
        <MessagesSection messages={messages} />
        <ToolsSection tools={tools} />
      </div>
    );
  }, [requestBody, hasContent]);

  if (!hasContent) {
    return (
      <Card title="请求内容" style={style}>
        <Text type="secondary">(无请求内容)</Text>
      </Card>
    );
  }

  return (
    <CollapsibleCard
      title="请求内容"
      headerExtraContent={
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <Text size="small" style={{ color: 'var(--semi-color-text-2)' }}>
            JSON 视图
          </Text>
          <Switch size="small" checked={isJsonViewMode} onChange={setIsJsonViewMode} />
        </div>
      }
      style={style}
    >
      {isJsonViewMode ? jsonView : (formattedView ?? <Text type="secondary">(无请求内容)</Text>)}
    </CollapsibleCard>
  );
}
