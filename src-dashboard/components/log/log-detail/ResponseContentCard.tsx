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
  /** API 类型（anthropic / openai），用于选择解析器 */
  api_type?: string;
  style?: React.CSSProperties;
}

/**
 * 根据 api_type 和 format 选择解析器解析响应体为 ContentBlockInfo[]。
 *
 * OpenAI 协议分支调用 parseOpenAI.ts 中的专用解析函数，
 * Anthropic 协议分支（或 api_type 为空）保持现有 parseStructuredBlocks 逻辑。
 */
function parseResponseByApiType(
  responseBody: string,
  format: ResponseFormat,
  api_type?: string,
): ContentBlockInfo[] {
  if (api_type === 'openai') {
    // OpenAI Chat Completions
    if (format === 'sse') {
      // 尝试 Chat Completions SSE 解析，失败则尝试 Responses SSE
      const chatBlocks = parseOpenAIChatSSE(responseBody);
      if (chatBlocks.length > 0) return chatBlocks;
      return parseOpenAIResponsesSSE(responseBody);
    }
    // 非流式：尝试 Chat Completions，失败则尝试 Responses
    const chatBlocks = parseOpenAIChatResponse(responseBody);
    if (chatBlocks.length > 0) return chatBlocks;
    return parseOpenAIResponsesResponse(responseBody);
  }

  // Anthropic（默认）
  return parseStructuredBlocks(responseBody, format).content_blocks;
}

/**
 * ResponseContentCard - 响应内容展示卡片
 *
 * 支持结构化视图（根据 api_type 和响应体格式选择解析器解析后按类型分组展示）和原始视图，
 * 通过 Switch 切换模式。通过响应头 Content-Type 判定 SSE 或 JSON 格式，
 * OpenAI 和 Anthropic 协议使用各自的解析路径但输出相同的 ContentBlockInfo 结构。
 */
export default function ResponseContentCard({
  responseBody,
  responseHeaders,
  api_type,
  style,
}: ResponseContentCardProps): ReactNode {
  const [viewMode, setViewMode] = useState<'formatted' | 'json'>('formatted');

  const hasBody = !!responseBody;

  // 通过响应头 Content-Type 检测格式
  const format: ResponseFormat = useMemo(
    () => detectResponseFormat(responseHeaders),
    [responseHeaders],
  );

  // 按 api_type 和 format 解析响应体
  const contentBlocks: ContentBlockInfo[] = useMemo(
    () => (responseBody ? parseResponseByApiType(responseBody, format, api_type) : []),
    [responseBody, format, api_type],
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
