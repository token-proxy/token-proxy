import { useMemo } from 'react';
import { Button, Collapsible, Tag, Typography } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import type { ConversationEvent, TokenUsage } from '../types/log.ts';
import { formatDateTime } from '../utils/format.ts';

const { Text, Paragraph } = Typography;

interface ClaudeSessionTimelineProps {
  events: ConversationEvent[];
  onOpenRaw: (logId: string) => void;
  tokenUsageMap?: Record<string, TokenUsage>;
}

function eventColor(event: ConversationEvent): 'blue' | 'green' | 'amber' | 'grey' | 'red' | 'violet' {
  if (event.event_type === 'user_message') return 'blue';
  if (event.event_type === 'assistant_thinking') return 'amber';
  if (event.event_type === 'tool_use') return 'violet';
  if (event.event_type === 'agent_call') return 'green';
  if (event.event_type === 'error') return 'red';
  return 'grey';
}

function eventTitle(event: ConversationEvent): string {
  if (event.title) return event.title;
  if (event.event_type === 'user_message') return '用户';
  if (event.event_type === 'assistant_message') return '助手';
  if (event.event_type === 'assistant_thinking') return '思考过程';
  if (event.event_type === 'agent_call') return `启动子代理${event.agent_type ? ` ${event.agent_type}` : ''}`;
  if (event.event_type === 'tool_use') return `工具调用${event.tool_name ? `: ${event.tool_name}` : ''}`;
  return event.event_type;
}

function renderPayload(payload?: Record<string, unknown> | null): ReactNode {
  if (!payload || Object.keys(payload).length === 0) return null;
  return (
    <pre
      style={{
        margin: '8px 0 0',
        padding: 12,
        borderRadius: 6,
        background: 'var(--semi-color-fill-0)',
        overflow: 'auto',
        whiteSpace: 'pre-wrap',
        wordBreak: 'break-word',
        fontSize: 12,
      }}
    >
      {JSON.stringify(payload, null, 2)}
    </pre>
  );
}

function renderTokenSummary(tokenUsage: TokenUsage): ReactNode {
  return (
    <div style={{ display: 'flex', gap: 8, marginBottom: 8, flexWrap: 'wrap' }}>
      <Tag size="small" color="light-blue">{`输入: ${tokenUsage.input_tokens.toLocaleString()}`}</Tag>
      <Tag size="small" color="light-blue">{`输出: ${tokenUsage.output_tokens.toLocaleString()}`}</Tag>
      {tokenUsage.cache_creation_input_tokens > 0 && (
        <Tag size="small" color="teal">{`缓存创建: ${tokenUsage.cache_creation_input_tokens.toLocaleString()}`}</Tag>
      )}
      {tokenUsage.cache_read_input_tokens > 0 && (
        <Tag size="small" color="teal">{`缓存读取: ${tokenUsage.cache_read_input_tokens.toLocaleString()}`}</Tag>
      )}
      {tokenUsage.thinking_tokens > 0 && (
        <Tag size="small" color="amber">{`思考: ${tokenUsage.thinking_tokens.toLocaleString()}`}</Tag>
      )}
      <Tag size="small">{`总计: ${tokenUsage.total_tokens.toLocaleString()}`}</Tag>
    </div>
  );
}

export default function ClaudeSessionTimeline({
  events,
  onOpenRaw,
  tokenUsageMap,
}: ClaudeSessionTimelineProps): ReactNode {
  // 按 request_index 分组事件，用于 token 汇总展示
  const groups = useMemo(() => {
    const g: { requestIndex: number; events: ConversationEvent[] }[] = [];
    let current: { requestIndex: number; events: ConversationEvent[] } | null = null;
    for (const event of events) {
      if (!current || current.requestIndex !== event.request_index) {
        current = { requestIndex: event.request_index, events: [] };
        g.push(current);
      }
      current.events.push(event);
    }
    return g;
  }, [events]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12, marginBottom: 24 }}>
      {groups.map((group) => {
        // 查找第一个事件的 token 用量（同一请求的所有事件共享 log_id）
        const firstEvent = group.events[0];
        const tokenUsage = tokenUsageMap?.[firstEvent.log_id];

        return (
          <div key={`group-${group.requestIndex}`}>
            {tokenUsage && renderTokenSummary(tokenUsage)}

            {group.events.map((event) => {
              const content = event.content || event.thinking_content;
              const isThinking = event.event_type === 'assistant_thinking';
              const isTool = event.event_type === 'tool_use' || event.event_type === 'agent_call';

              return (
                <div
                  key={event.id}
                  style={{
                    border: '1px solid var(--semi-color-border)',
                    borderRadius: 10,
                    padding: 14,
                    marginBottom: 8,
                    background: event.role === 'user'
                      ? 'var(--semi-color-primary-light-default)'
                      : 'var(--semi-color-bg-1)',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                    <Tag color={eventColor(event)}>{eventTitle(event)}</Tag>
                    {event.source === 'subagent' && <Tag color="green">子代理</Tag>}
                    {event.agent_type && <Tag>{event.agent_type}</Tag>}
                    {event.tool_name && <Tag>{event.tool_name}</Tag>}
                    <Text type="secondary" size="small">{formatDateTime(event.timestamp)}</Text>
                  </div>

                  {isThinking || isTool ? (
                    <Collapsible keepDOM>
                      <Paragraph
                        style={{
                          whiteSpace: 'pre-wrap',
                          wordBreak: 'break-word',
                          color: isThinking ? 'var(--semi-color-text-1)' : undefined,
                          marginBottom: 0,
                          lineHeight: 1.7,
                        }}
                      >
                        {(content || event.tool_result_content) ?? '(空)'}
                      </Paragraph>
                      {isTool && renderPayload(event.display_payload)}
                    </Collapsible>
                  ) : (
                    content && (
                      <Paragraph
                        style={{
                          whiteSpace: 'pre-wrap',
                          wordBreak: 'break-word',
                          marginBottom: 0,
                          lineHeight: 1.7,
                        }}
                      >
                        {content}
                      </Paragraph>
                    )
                  )}

                  <div style={{ marginTop: 10 }}>
                    <Button size="small" type="tertiary" onClick={() => onOpenRaw(event.log_id)}>
                      查看原始内容
                    </Button>
                  </div>
                </div>
              );
            })}
          </div>
        );
      })}
    </div>
  );
}