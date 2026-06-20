import { type ReactNode, useState } from 'react';
import { Button, Collapsible, Tag, Typography, Tooltip } from '@douyinfe/semi-ui';
import type { ConversationTurn, TurnBlock, TurnTokenSummary } from '../../types/log.ts';
import { formatDateTime, formatNumber } from '../../utils/format.ts';

const { Text, Paragraph } = Typography;

/** TurnCard 组件 Props */
interface TurnCardProps {
  /** 对话轮次数据 */
  turn: ConversationTurn;
  /** 打开原始内容的回调 */
  onOpenRaw: (logId: string) => void;
  /** 是否默认展开所有折叠区域 */
  defaultExpanded?: boolean;
}

// --- Token 展示 ---

/** 渲染 Token 小计标签，hover 时展示六维详情 */
function renderTokenCompact(summary: TurnTokenSummary): ReactNode {
  const tooltipContent = (
    <div style={{ whiteSpace: 'nowrap', lineHeight: 2 }}>
      <div>输入: {formatNumber(summary.inputTokens)}</div>
      <div>输出: {formatNumber(summary.outputTokens)}</div>
      <div>缓存创建: {formatNumber(summary.cacheCreationTokens)}</div>
      <div>缓存读取: {formatNumber(summary.cacheReadTokens)}</div>
      <div>思考: {formatNumber(summary.thinkingTokens)}</div>
      <div>总计: {formatNumber(summary.totalTokens)}</div>
    </div>
  );

  return (
    <Tooltip content={tooltipContent}>
      <Tag size="small" color="light-blue">
        ↑{formatNumber(summary.inputTokens)} ↓{formatNumber(summary.outputTokens)}
      </Tag>
    </Tooltip>
  );
}

// --- 辅助样式常量 ---

const blockCardStyle: React.CSSProperties = {
  border: '1px solid var(--semi-color-border)',
  borderRadius: 10,
  padding: 14,
  background: 'var(--semi-color-bg-1)',
};

const blockHeaderStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 8,
  marginBottom: 8,
};

const preStyle: React.CSSProperties = {
  margin: 0,
  padding: 12,
  borderRadius: 6,
  background: 'var(--semi-color-fill-0)',
  overflow: 'auto',
  whiteSpace: 'pre-wrap',
  wordBreak: 'break-word',
  fontSize: 12,
  fontFamily: '"SF Mono", "Monaco", "Cascadia Code", Menlo, monospace',
  lineHeight: 1.7,
};

const paragraphStyle: React.CSSProperties = {
  whiteSpace: 'pre-wrap',
  wordBreak: 'break-word',
  marginBottom: 0,
  lineHeight: 1.7,
};

// --- 单个 block 渲染函数 ---

/** 渲染 thinking 块 */
function renderThinkingBlock(block: TurnBlock & { type: 'thinking' }, defaultExpanded?: boolean): ReactNode {
  return (
    <div key={`think-${block.logId}-${block.timestamp}`} style={{ marginBottom: 12 }}>
      <Collapsible keepDOM defaultActiveKey={defaultExpanded ? '' : 'thinking'}>
        <div style={blockCardStyle}>
          <div style={blockHeaderStyle}>
            <Tag color="amber">思考过程</Tag>
            <Text type="secondary" size="small">{formatDateTime(block.timestamp)}</Text>
          </div>
          <pre style={{ ...preStyle, whiteSpace: 'pre-wrap' }}>
            {block.content}
          </pre>
        </div>
      </Collapsible>
    </div>
  );
}

/** 渲染 tool_use 块 */
function renderToolUseBlock(block: TurnBlock & { type: 'tool_use' }, defaultExpanded?: boolean): ReactNode {
  return (
    <div key={`tool-${block.logId}-${block.timestamp}`} style={{ marginBottom: 12 }}>
      <Collapsible keepDOM defaultActiveKey={defaultExpanded ? '' : 'tool_use'}>
        <div style={blockCardStyle}>
          <div style={blockHeaderStyle}>
            <Tag color="violet">工具调用: {block.toolName}</Tag>
            <Text type="secondary" size="small">{formatDateTime(block.timestamp)}</Text>
          </div>
          {block.input && Object.keys(block.input).length > 0 && (
            <pre style={preStyle}>
              {JSON.stringify(block.input, null, 2)}
            </pre>
          )}
        </div>
      </Collapsible>
    </div>
  );
}

/** 渲染 tool_result 块（默认折叠，截断前 200 字符） */
function renderToolResultBlock(block: TurnBlock & { type: 'tool_result' }, defaultExpanded?: boolean): ReactNode {
  // 截断显示前 200 字符
  const truncated = block.content.length > 200
    ? block.content.slice(0, 200) + '...'
    : block.content;

  return (
    <div key={`result-${block.logId}-${block.timestamp}`} style={{ marginBottom: 12 }}>
      <Collapsible keepDOM defaultActiveKey={defaultExpanded ? '' : 'tool_result'}>
        <div style={blockCardStyle}>
          <div style={blockHeaderStyle}>
            <Tag color="grey">工具结果</Tag>
            {block.isError && <Tag color="red">错误</Tag>}
            <Text type="secondary" size="small">{formatDateTime(block.timestamp)}</Text>
          </div>
          {block.content.length > 200 && (
            <>
              <pre style={{ ...preStyle, marginBottom: 8 }}>{truncated}</pre>
              <Text type="secondary" size="small" style={{ display: 'block', textAlign: 'right' }}>
                (共 {block.content.length} 字符, 已截断)
              </Text>
            </>
          )}
          {block.content.length <= 200 && (
            <pre style={preStyle}>{block.content}</pre>
          )}
        </div>
      </Collapsible>
    </div>
  );
}

/** 渲染 assistant_message 块（直接展示，不折叠） */
function renderAssistantMessageBlock(block: TurnBlock & { type: 'assistant_message' }): ReactNode {
  return (
    <div key={`msg-${block.logId}-${block.timestamp}`} style={{ marginBottom: 12 }}>
      <div style={blockCardStyle}>
        <div style={blockHeaderStyle}>
          <Tag color="blue">助手</Tag>
          <Text type="secondary" size="small">{formatDateTime(block.timestamp)}</Text>
        </div>
        {block.content && (
          <Paragraph style={paragraphStyle}>
            {block.content}
          </Paragraph>
        )}
      </div>
    </div>
  );
}

/** 渲染 agent_call 块（默认折叠，递归渲染 children） */
function renderAgentCallBlock(block: TurnBlock & { type: 'agent_call' }, defaultExpanded?: boolean): ReactNode {
  return (
    <div key={`agent-${block.logId}-${block.timestamp}`} style={{ marginBottom: 12 }}>
      <Collapsible keepDOM defaultActiveKey={defaultExpanded ? '' : 'agent_call'}>
        <div style={blockCardStyle}>
          <div style={blockHeaderStyle}>
            <Tag color="green">Agent: {block.agentType}</Tag>
            <Text type="secondary" size="small">{formatDateTime(block.timestamp)}</Text>
          </div>

          {/* 摘要信息：子事件数量 + Token 小计 */}
          <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginBottom: 8 }}>
            <Tag size="small">{block.children.length} 个事件</Tag>
            {renderTokenCompact(block.tokenSummary)}
          </div>

          {/* 递归渲染子 block */}
          <div
            style={{
              borderLeft: '2px solid var(--semi-color-border)',
              paddingLeft: 16,
              background: 'var(--semi-color-fill-0)',
              borderRadius: 6,
              padding: 12,
            }}
          >
            {block.children.map((child) => renderTurnBlock(child, defaultExpanded))}
          </div>
        </div>
      </Collapsible>
    </div>
  );
}

// --- TurnBlock 分发渲染 ---

/**
 * 根据 TurnBlock 类型分发到对应的渲染函数
 *
 * 递归调用：agent_call 类型的 blocks 会递归渲染其 children。
 */
function renderTurnBlock(block: TurnBlock, defaultExpanded?: boolean): ReactNode {
  switch (block.type) {
    case 'thinking':
      return renderThinkingBlock(block, defaultExpanded);
    case 'tool_use':
      return renderToolUseBlock(block, defaultExpanded);
    case 'tool_result':
      return renderToolResultBlock(block, defaultExpanded);
    case 'assistant_message':
      return renderAssistantMessageBlock(block);
    case 'agent_call':
      return renderAgentCallBlock(block, defaultExpanded);
    default:
      return null;
  }
}

// --- TurnCard 主组件 ---

/**
 * TurnCard - 对话轮次卡片组件
 *
 * 渲染单个对话轮次的所有内容，包括用户消息、思考过程、工具调用、
 * 工具结果、助手回复和子代理调用，以及 Token 用量摘要。
 * 支持折叠/展开和查看原始内容。
 */
export default function TurnCard({
  turn,
  onOpenRaw,
  defaultExpanded = true,
}: TurnCardProps): ReactNode {
  const [collapsed, setCollapsed] = useState(!defaultExpanded);

  return (
    <div
      style={{
        border: '1px solid var(--semi-color-border)',
        borderRadius: 12,
        background: 'var(--semi-color-bg-0)',
        overflow: 'hidden',
      }}
    >
      {/* ─── 标题栏 ─── */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '10px 16px',
          background: 'var(--semi-color-fill-0)',
          borderBottom: '1px solid var(--semi-color-border)',
          flexWrap: 'wrap',
        }}
      >
        <Text strong style={{ fontSize: 14 }}>
          轮次 {turn.turnIndex}
        </Text>
        <Text type="secondary" size="small">
          {formatDateTime(turn.startTime)}
        </Text>
        {renderTokenCompact(turn.tokenSummary)}
        <Text type="secondary" size="small">
          {turn.logIds.length} 次请求
        </Text>
      </div>

      {/* ─── 可折叠主体 ─── */}
      <Collapsible keepDOM isOpen={!collapsed}>
        <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 4 }}>
          {/* 用户消息 */}
          <div
            style={{
              borderRadius: 10,
              padding: 14,
              marginBottom: 8,
              background: 'var(--semi-color-primary-light-default)',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
              <Tag color="blue">用户</Tag>
              <Text type="secondary" size="small">{formatDateTime(turn.startTime)}</Text>
            </div>
            <Paragraph style={paragraphStyle}>
              {turn.userMessage}
            </Paragraph>
          </div>

          {/* 按顺序渲染 blocks */}
          {turn.blocks.map((block) => renderTurnBlock(block, defaultExpanded))}
        </div>
      </Collapsible>

      {/* ─── 底部操作栏 ─── */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          padding: '8px 16px',
          borderTop: '1px solid var(--semi-color-border)',
          background: 'var(--semi-color-fill-0)',
        }}
      >
        <Button size="small" type="tertiary" onClick={() => onOpenRaw(turn.logIds[0])}>
          查看原始内容
        </Button>
        <Button
          size="small"
          type="tertiary"
          onClick={() => setCollapsed((v) => !v)}
        >
          {collapsed ? '展开轮次' : '收起轮次'}
        </Button>
      </div>
    </div>
  );
}
