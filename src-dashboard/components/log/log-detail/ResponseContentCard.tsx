import { type ReactNode, useCallback, useMemo, useState } from 'react';
import { Collapse, Switch, Typography } from '@douyinfe/semi-ui';
import CollapsibleCard from '@components/common/CollapsibleCard';
import RawResponseView from '@components/log/RawResponseView';
import { parseStructuredBlocks, detectResponseFormat } from '../../../utils/parseLogs.ts';
import type { ResponseFormat, ContentBlockInfo } from '../../../utils/parseLogs.ts';
import {
  parseOpenAIChatResponse,
  parseOpenAIChatSSE,
  parseOpenAIResponsesResponse,
  parseOpenAIResponsesSSE,
} from '../../../utils/parseOpenAI.ts';
import ThinkingBlockCard from './response-content/ThinkingBlockCard';
import TextBlockCard from './response-content/TextBlockCard';
import ToolUseBlockCard from './response-content/ToolUseBlockCard';

const { Text } = Typography;

/** ResponseContentCard 组件 Props */
interface ResponseContentCardProps {
  responseBody: string | null | undefined;
  /** 响应头，用于通过 Content-Type 检测响应体格式 */
  responseHeaders?: Record<string, unknown> | null;
  /** API 类型（anthropic / openai），仅在 api_protocol 缺失时用于选择解析器 */
  api_type?: string;
  /** 请求级协议（anthropic / openai / openai_response），存在时优先于 api_type 选择解析器 */
  api_protocol?: string;
  style?: React.CSSProperties;
}

/**
 * 根据 api_protocol / api_type 和 format 选择解析器解析响应体为 ContentBlockInfo[]。
 *
 * 协议分发优先级：api_protocol（新字段）优先，缺失时回退 api_type（旧数据兼容）。
 * - openai_response：直接调用 Responses API 解析器，不再试探
 * - openai：直接调用 Chat Completions 解析器，Responses 解析器仅作安全兜底
 * - 其余（含 anthropic 或协议字段为空）：保持现有 parseStructuredBlocks 逻辑
 */
function parseResponseByApiType(
  responseBody: string,
  format: ResponseFormat,
  api_type?: string,
  api_protocol?: string,
): ContentBlockInfo[] {
  // api_protocol 缺失时按旧规则回退到 api_type === 'openai'
  const protocol = api_protocol ?? (api_type === 'openai' ? 'openai' : undefined);

  // 1. Responses API：协议明确，直接调用对应解析器
  if (protocol === 'openai_response') {
    return format === 'sse'
      ? parseOpenAIResponsesSSE(responseBody)
      : parseOpenAIResponsesResponse(responseBody);
  }

  // 2. Chat Completions：直接调用 Chat 解析器，解析为空时用 Responses 兜底
  if (protocol === 'openai') {
    const chatBlocks =
      format === 'sse' ? parseOpenAIChatSSE(responseBody) : parseOpenAIChatResponse(responseBody);
    if (chatBlocks.length > 0) return chatBlocks;
    return format === 'sse'
      ? parseOpenAIResponsesSSE(responseBody)
      : parseOpenAIResponsesResponse(responseBody);
  }

  // 3. Anthropic（默认）
  return parseStructuredBlocks(responseBody, format).content_blocks;
}

/**
 * ResponseContentCard - 响应内容展示卡片
 *
 * 支持结构化视图（根据 api_protocol / api_type 和响应体格式选择解析器解析后按类型分组展示）和原始视图，
 * 通过 Switch 切换模式。通过响应头 Content-Type 判定 SSE 或 JSON 格式，
 * OpenAI 和 Anthropic 协议使用各自的解析路径但输出相同的 ContentBlockInfo 结构。
 */
export default function ResponseContentCard({
  responseBody,
  responseHeaders,
  api_type,
  api_protocol,
  style,
}: ResponseContentCardProps): ReactNode {
  const [viewMode, setViewMode] = useState<'formatted' | 'json'>('formatted');

  const hasBody = !!responseBody;

  // 通过响应头 Content-Type 检测格式
  const format: ResponseFormat = useMemo(
    () => detectResponseFormat(responseHeaders),
    [responseHeaders],
  );

  // 按 api_protocol / api_type 和 format 解析响应体
  const contentBlocks: ContentBlockInfo[] = useMemo(
    () =>
      responseBody ? parseResponseByApiType(responseBody, format, api_type, api_protocol) : [],
    [responseBody, format, api_type, api_protocol],
  );

  // 默认展开 text 和 tool_use 类型的 block（助手回复和工具调用）
  const defaultActiveKeys = useMemo<string[]>(() => {
    return contentBlocks
      .map((block, idx) =>
        block.block_type === 'text' || block.block_type === 'tool_use' ? String(idx) : null,
      )
      .filter((k): k is string => k !== null);
  }, [contentBlocks]);

  // 结构化视图 JSX 树
  const structuredView = useMemo<ReactNode>(() => {
    if (!hasBody || contentBlocks.length === 0) {
      return <Text type="secondary">(无响应内容)</Text>;
    }

    // 汇总信息：统计各类型 block 和字符数
    const thinkingBlocks = contentBlocks.filter((b) => b.block_type === 'thinking');
    const textBlocks = contentBlocks.filter((b) => b.block_type === 'text');
    const toolBlocks = contentBlocks.filter((b) => b.block_type === 'tool_use');
    const thinkingChars = thinkingBlocks.reduce((s, b) => s + (b.thinking?.length || 0), 0);
    const textChars = textBlocks.reduce((s, b) => s + (b.text?.length || 0), 0);

    const parts: string[] = [];
    if (thinkingBlocks.length > 0)
      parts.push(`推理 ${thinkingBlocks.length} 块 (${thinkingChars.toLocaleString()} 字)`);
    if (textBlocks.length > 0)
      parts.push(`助手回复 ${textBlocks.length} 块 (${textChars.toLocaleString()} 字)`);
    if (toolBlocks.length > 0) parts.push(`工具调用 ${toolBlocks.length} 个`);

    return (
      <div>
        {parts.length > 0 && (
          <Text type="tertiary" size="small" style={{ display: 'block', marginBottom: 12 }}>
            {parts.join(' · ')}
          </Text>
        )}
        <Collapse defaultActiveKey={defaultActiveKeys}>
          {contentBlocks.map((block, idx) => {
            const itemKey = String(idx);
            switch (block.block_type) {
              case 'thinking':
                return <ThinkingBlockCard key={itemKey} block={block} itemKey={itemKey} />;
              case 'text':
                return <TextBlockCard key={itemKey} block={block} itemKey={itemKey} />;
              case 'tool_use':
                return <ToolUseBlockCard key={itemKey} block={block} itemKey={itemKey} />;
              default:
                return null;
            }
          })}
        </Collapse>
      </div>
    );
  }, [hasBody, contentBlocks, defaultActiveKeys]);

  // 原始视图 JSX 树 — 延迟渲染（仅当用户切换到原始视图时才解析大文本）
  const [rawRendered, setRawRendered] = useState(false);
  const rawView = useMemo<ReactNode>(() => {
    if (!hasBody) {
      return <Text type="secondary">(无响应内容)</Text>;
    }
    if (!rawRendered) return null;
    return <RawResponseView body={responseBody!} />;
  }, [hasBody, responseBody, rawRendered]);

  // 切换到原始视图时触发延迟渲染
  const handleViewModeChange = useCallback((checked: boolean) => {
    const newMode = checked ? 'json' : 'formatted';
    setViewMode(newMode);
    if (newMode === 'json') setRawRendered(true);
  }, []);

  // 原始视图标签（根据格式动态显示）
  const rawLabel = format === 'sse' ? '原始 SSE' : '原始 JSON';

  return (
    <CollapsibleCard
      title="响应内容"
      headerExtraContent={
        hasBody ? (
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Text size="small" style={{ color: 'var(--semi-color-text-2)' }}>
              {viewMode === 'formatted' ? '结构化' : rawLabel}
            </Text>
            <Switch size="small" checked={viewMode === 'json'} onChange={handleViewModeChange} />
          </div>
        ) : undefined
      }
      defaultCollapsed={false}
      style={style}
    >
      {viewMode === 'formatted' ? structuredView : rawView}
    </CollapsibleCard>
  );
}
