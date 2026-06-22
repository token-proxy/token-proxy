import { type ReactNode, useMemo, useState } from 'react';
import { Collapse, Switch, Typography } from '@douyinfe/semi-ui';
import CollapsibleCard from '@components/common/CollapsibleCard';
import RawResponseView from '@components/log/RawResponseView';
import { parseStructuredBlocks, detectResponseFormat } from '../../../utils/parseLogs.ts';
import type { ResponseFormat } from '../../../utils/parseLogs.ts';
import ThinkingBlockCard from './response-content/ThinkingBlockCard';
import TextBlockCard from './response-content/TextBlockCard';
import ToolUseBlockCard from './response-content/ToolUseBlockCard';

const { Text } = Typography;

/** ResponseContentCard 组件 Props */
interface ResponseContentCardProps {
  responseBody: string | null | undefined;
  /** 响应头，用于通过 Content-Type 检测响应体格式 */
  responseHeaders?: Record<string, unknown> | null;
  style?: React.CSSProperties;
}

/**
 * ResponseContentCard - 响应内容展示卡片
 *
 * 支持结构化视图（根据响应体格式解析 content block 后按类型分组展示）和原始视图，
 * 通过 Switch 切换模式。通过响应头 Content-Type 判定 SSE 或 JSON 格式，
 * 不同格式使用不同解析路径但输出相同的 ContentBlockInfo 结构。
 */
export default function ResponseContentCard({
  responseBody,
  responseHeaders,
  style,
}: ResponseContentCardProps): ReactNode {
  const [viewMode, setViewMode] = useState<'formatted' | 'json'>('formatted');

  const hasBody = !!responseBody;

  // 通过响应头 Content-Type 检测格式
  const format: ResponseFormat = useMemo(
    () => detectResponseFormat(responseHeaders),
    [responseHeaders],
  );

  // 解析结构化数据，传入格式避免内部重复检测
  const parsed = useMemo(
    () => (responseBody ? parseStructuredBlocks(responseBody, format) : null),
    [responseBody, format],
  );

  const hasContentBlocks = parsed && parsed.content_blocks.length > 0;
  const hasMessageStart = !!parsed?.message_start;
  const hasMessageDelta = !!parsed?.message_delta;

  // 默认展开 text 和 tool_use 类型的 block (助手回复和工具调用)
  const defaultActiveKeys = useMemo<string[]>(() => {
    if (!parsed) return [];
    return parsed.content_blocks
      .map((block, idx) =>
        block.block_type === 'text' || block.block_type === 'tool_use' ? String(idx) : null,
      )
      .filter((k): k is string => k !== null);
  }, [parsed]);

  // 结构化视图 JSX 树
  const structuredView = useMemo<ReactNode>(() => {
    if (!hasBody || !parsed) {
      return <Text type="secondary">(无响应内容)</Text>;
    }

    if (!hasContentBlocks && !hasMessageStart && !hasMessageDelta) {
      return <Text type="secondary">(无响应内容)</Text>;
    }

    return (
      <Collapse defaultActiveKey={defaultActiveKeys}>
        {/* 块 B/C/D: 按 index 顺序渲染 content blocks */}
        {parsed.content_blocks.map((block, idx) => {
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
    );
  }, [hasBody, parsed, hasContentBlocks, hasMessageStart, hasMessageDelta, defaultActiveKeys]);

  // 原始视图 JSX 树
  const rawView = useMemo<ReactNode>(() => {
    if (!hasBody) {
      return <Text type="secondary">(无响应内容)</Text>;
    }
    return <RawResponseView body={responseBody!} />;
  }, [hasBody, responseBody]);

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
            <Switch
              size="small"
              checked={viewMode === 'json'}
              onChange={(checked) => setViewMode(checked ? 'json' : 'formatted')}
            />
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
