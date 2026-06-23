import { type ReactNode, useMemo, useState } from 'react';
import { Card, Descriptions, Switch, Tag, Typography } from '@douyinfe/semi-ui';
import CodeHighlight from '@components/common/CodeHighlight';
import CollapsibleCard from '@components/common/CollapsibleCard';
import RequestConfigSection from './request-content/RequestConfigSection';
import SystemPromptSection from './request-content/SystemPromptSection';
import MessagesSection from './request-content/MessagesSection';
import ToolsSection from './request-content/ToolsSection';
import AccordionSection from './request-content/AccordionSection';
import ExpandableContentBlock from '@components/common/ExpandableContentBlock';
import { parseOpenAIRequestBody } from '../../../utils/parseOpenAI.ts';
import type {
  OpenAIChatRequest,
  OpenAIChatMessage,
  OpenAIResponsesRequest,
} from '../../../utils/parseOpenAI.ts';
import './RequestContentCard.css';

const { Text } = Typography;

/** RequestContentCard 组件 Props */
interface RequestContentCardProps {
  requestBody: Record<string, unknown> | null | undefined;
  /** API 类型（anthropic / openai），用于选择解析器 */
  api_type?: string;
  style?: React.CSSProperties;
}

/**
 * RequestContentCard - 请求内容展示卡片
 *
 * 支持结构化视图（按 api_type 分段展示模型配置、系统提示词、消息、工具）和原始 JSON 视图，
 * 通过 Switch 切换模式。
 * - Anthropic：三段式（RequestConfig + SystemPrompt + Messages + Tools）
 * - OpenAI：Chat Completions / Responses API 两种格式自适应
 */
export default function RequestContentCard({
  requestBody,
  api_type,
  style,
}: RequestContentCardProps): ReactNode {
  const [isJsonViewMode, setIsJsonViewMode] = useState(false);

  const hasContent = requestBody != null && Object.keys(requestBody).length > 0;

  // 解析 OpenAI 请求体（仅当 api_type === 'openai' 时）
  const openaiParsed = useMemo(() => {
    if (api_type !== 'openai' || !requestBody) return null;
    return parseOpenAIRequestBody(requestBody);
  }, [api_type, requestBody]);

  // isOpenAI 判定
  const isOpenAI = api_type === 'openai' && openaiParsed !== null;

  // 原始 JSON 视图：用 useMemo 缓存 JSX 元素树
  const jsonView = useMemo(() => {
    if (!hasContent) return null;
    const jsonText = JSON.stringify(requestBody, null, 2);
    return <CodeHighlight content={jsonText} />;
  }, [requestBody, hasContent]);

  // 结构化视图：用 useMemo 缓存 JSX 元素树
  const formattedView = useMemo(() => {
    if (!hasContent || !requestBody) return null;

    // OpenAI 协议：使用专用解析和渲染
    if (isOpenAI && openaiParsed) {
      return (
        <div>
          {openaiParsed.kind === 'chat' ? (
            <OpenAIChatRequestView parsed={openaiParsed} />
          ) : (
            <OpenAIResponsesRequestView parsed={openaiParsed} />
          )}
        </div>
      );
    }

    // Anthropic（默认）：三段式渲染
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
  }, [requestBody, hasContent, isOpenAI, openaiParsed]);

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

// ─── OpenAI Chat Completions 请求视图 ───

/** Chat Completions 请求参数标签映射 */
const CHAT_PARAM_LABELS: Record<string, string> = {
  model: '模型',
  temperature: '温度',
  max_tokens: '最大 Token 数',
  max_completion_tokens: '最大完成 Token 数',
  top_p: 'Top P',
  frequency_penalty: '频率惩罚',
  presence_penalty: '存在惩罚',
  stream: '流式',
  seed: '随机种子',
};

/** 单条 OpenAI Chat 消息渲染块 */
function OpenAIChatMessageBlock({ message }: { message: OpenAIChatMessage }): ReactNode {
  const roleLabel =
    message.role === 'system'
      ? '系统'
      : message.role === 'user'
        ? '用户'
        : message.role === 'assistant'
          ? '助手'
          : message.role === 'tool'
            ? '工具'
            : message.role;
  const roleColor =
    message.role === 'system'
      ? 'grey'
      : message.role === 'user'
        ? 'blue'
        : message.role === 'assistant'
          ? 'green'
          : message.role === 'tool'
            ? 'orange'
            : undefined;

  const cardStyle: React.CSSProperties = {
    marginBottom: 12,
    padding: 12,
    border: '1px solid var(--semi-color-border)',
    borderRadius: 6,
  };

  const headerStyle: React.CSSProperties = {
    marginBottom: 8,
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    flexWrap: 'wrap',
  };

  return (
    <div style={cardStyle}>
      <div style={headerStyle}>
        <Tag size="small" color={roleColor}>
          {roleLabel}
        </Tag>
        {message.name && (
          <Tag size="small" type="light">
            {message.name}
          </Tag>
        )}
        {message.tool_call_id && (
          <Tag size="small" color="green">
            工具结果 ({message.tool_call_id})
          </Tag>
        )}
      </div>
      {/* 文本内容 */}
      {typeof message.content === 'string' && message.content.length > 0 && (
        <ExpandableContentBlock content={message.content} collapseLabel="收起消息" />
      )}
      {/* 工具调用 */}
      {message.tool_calls &&
        message.tool_calls.length > 0 &&
        message.tool_calls.map((tc) => {
          let argsStr = tc.function.arguments;
          try {
            argsStr = JSON.stringify(JSON.parse(tc.function.arguments), null, 2);
          } catch {
            // 保持原样
          }
          return (
            <div key={tc.id} style={{ marginTop: 8 }}>
              <div style={{ marginBottom: 4 }}>
                <Tag size="small" color="orange">
                  工具调用: {tc.function.name}
                </Tag>
              </div>
              <CodeHighlight content={argsStr} language="json" />
            </div>
          );
        })}
    </div>
  );
}

/** OpenAI Chat Completions 请求体结构化视图 */
function OpenAIChatRequestView({ parsed }: { parsed: OpenAIChatRequest }): ReactNode {
  const { messages, tools, params } = parsed;

  // 请求配置项
  const configItems: Array<{ key: string; value: string }> = [];
  for (const [k, v] of Object.entries(params)) {
    if (k === 'messages' || k === 'tools') continue;
    const label = CHAT_PARAM_LABELS[k] || k;
    if (typeof v === 'boolean') {
      configItems.push({ key: label, value: v ? '是' : '否' });
    } else if (v != null && typeof v !== 'object') {
      configItems.push({ key: label, value: String(v) });
    }
  }

  // 分离系统消息和其他消息
  const systemMsgs = messages.filter((m) => m.role === 'system');
  const otherMsgs = messages.filter((m) => m.role !== 'system');

  return (
    <div>
      {/* 请求配置 */}
      {configItems.length > 0 && (
        <AccordionSection title="请求配置" defaultExpanded={false}>
          <Descriptions
            row
            size="small"
            data={configItems.map((d) => ({ key: d.key, value: d.value }))}
          />
        </AccordionSection>
      )}

      {/* 系统消息 */}
      {systemMsgs.length > 0 && (
        <AccordionSection title={`系统消息（共 ${systemMsgs.length} 条）`} defaultExpanded={false}>
          {systemMsgs.map((msg, idx) => (
            <OpenAIChatMessageBlock key={idx} message={msg} />
          ))}
        </AccordionSection>
      )}

      {/* 对话消息 */}
      {otherMsgs.length > 0 && (
        <AccordionSection title={`消息（共 ${otherMsgs.length} 条）`}>
          {otherMsgs.map((msg, idx) => (
            <OpenAIChatMessageBlock key={idx} message={msg} />
          ))}
        </AccordionSection>
      )}

      {/* 工具定义 */}
      {tools && tools.length > 0 && (
        <AccordionSection title={`工具定义（共 ${tools.length} 个）`} defaultExpanded={false}>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
            {tools.map((tool, idx) => (
              <Tag key={idx} size="small" color="grey">
                {tool.function.name}
              </Tag>
            ))}
          </div>
        </AccordionSection>
      )}
    </div>
  );
}

/** OpenAI Responses API 请求体结构化视图 */
function OpenAIResponsesRequestView({ parsed }: { parsed: OpenAIResponsesRequest }): ReactNode {
  const { model, input, instructions, tools, params } = parsed;

  // 请求配置项
  const configItems: Array<{ key: string; value: string }> = [];
  for (const [k, v] of Object.entries(params)) {
    if (k === 'input' || k === 'instructions' || k === 'tools' || k === 'model') continue;
    if (typeof v === 'boolean') {
      configItems.push({ key: k, value: v ? '是' : '否' });
    } else if (v != null && typeof v !== 'object') {
      configItems.push({ key: k, value: String(v) });
    }
  }

  return (
    <div>
      {/* 模型 */}
      {model && (
        <AccordionSection title="请求配置" defaultExpanded={false}>
          <Descriptions
            row
            size="small"
            data={[
              { key: '模型', value: model },
              ...configItems.map((d) => ({ key: d.key, value: d.value })),
            ]}
          />
        </AccordionSection>
      )}

      {/* Instructions */}
      {instructions && (
        <AccordionSection title="指示" defaultExpanded={false}>
          <ExpandableContentBlock content={instructions} collapseLabel="收起指示" />
        </AccordionSection>
      )}

      {/* Input */}
      {input.length > 0 && (
        <AccordionSection title={`输入（共 ${input.length} 项）`}>
          <CodeHighlight content={JSON.stringify(input, null, 2)} language="json" />
        </AccordionSection>
      )}

      {/* 工具定义 */}
      {tools && tools.length > 0 && (
        <AccordionSection title={`工具定义（共 ${tools.length} 个）`} defaultExpanded={false}>
          <CodeHighlight content={JSON.stringify(tools, null, 2)} language="json" />
        </AccordionSection>
      )}
    </div>
  );
}
