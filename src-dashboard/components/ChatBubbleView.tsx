import { Button, Tag, Typography } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import type { LogDetail } from '../types/log.ts';
import { formatDateTime, formatDuration } from '../utils/format.ts';

const { Text } = Typography;

interface ChatBubbleViewProps {
  details: LogDetail[];
  onOpenRaw: (title: string, content: string) => void;
}

function extractLastUserMessage(requestBody?: Record<string, unknown> | null): string {
  if (!requestBody) return '(无请求体)';
  const body = requestBody as Record<string, unknown>;
  const messages = body.messages;
  if (Array.isArray(messages)) {
    const userMsgs = messages.filter((m: Record<string, unknown>) => m.role === 'user');
    if (userMsgs.length > 0) {
      const last = userMsgs[userMsgs.length - 1] as Record<string, unknown>;
      if (typeof last.content === 'string') return last.content;
      if (Array.isArray(last.content)) {
        return (last.content as Array<Record<string, unknown>>)
          .map((c: Record<string, unknown>) => typeof c.text === 'string' ? c.text : '')
          .filter(Boolean)
          .join('');
      }
      return JSON.stringify(last.content);
    }
  }
  return JSON.stringify(requestBody, null, 2);
}

function extractTextFromSSE(sse: string): string {
  const lines = sse.split('\n');
  const texts: string[] = [];

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith('data:')) {
      const data = trimmed.slice(5).trim();
      if (data === '[DONE]') continue;

      try {
        const parsed = JSON.parse(data) as Record<string, unknown>;
        const choices = parsed.choices;
        if (Array.isArray(choices)) {
          for (const c of choices as Array<Record<string, unknown>>) {
            const delta = c.delta as Record<string, unknown> | undefined;
            if (delta?.content && typeof delta.content === 'string') texts.push(delta.content);
            else if (delta?.text && typeof delta.text === 'string') texts.push(delta.text);
            else if (c.text && typeof c.text === 'string') texts.push(c.text);
          }
        } else if (parsed.type === 'content_block_delta') {
          const delta = parsed.delta as Record<string, unknown> | undefined;
          if (delta?.text && typeof delta.text === 'string') texts.push(delta.text);
        } else if (parsed.content && typeof parsed.content === 'string') {
          texts.push(parsed.content);
        }
      } catch {
        texts.push(data);
      }
    }
  }

  return texts.join('');
}

function extractAssistantResponse(responseBody?: string | null): string {
  if (!responseBody) return '(无响应体)';

  try {
    const parsed = JSON.parse(responseBody) as Record<string, unknown>;
    const choices = parsed.choices;
    if (Array.isArray(choices)) {
      const text = (choices as Array<Record<string, unknown>>)
        .map((c: Record<string, unknown>) => {
          const msg = c.message as Record<string, unknown> | undefined;
          if (msg?.content && typeof msg.content === 'string') return msg.content;
          if (c.text && typeof c.text === 'string') return c.text;
          return '';
        })
        .filter(Boolean)
        .join('');
      if (text) return text;
    }
    const content = parsed.content;
    if (Array.isArray(content)) {
      const text = (content as Array<Record<string, unknown>>)
        .map((c: Record<string, unknown>) => typeof c.text === 'string' ? c.text : '')
        .filter(Boolean)
        .join('');
      if (text) return text;
    }
    if (typeof content === 'string') return content;

    return JSON.stringify(parsed, null, 2);
  } catch {
    const extracted = extractTextFromSSE(responseBody);
    return extracted || responseBody;
  }
}

export default function ChatBubbleView({ details, onOpenRaw }: ChatBubbleViewProps): ReactNode {
  return (
    <div style={{ marginBottom: 32 }}>
      {details.map((detail, index) => {
        const userMsg = extractLastUserMessage(detail.request_body);
        const assistantMsg = extractAssistantResponse(detail.response_body);
        return (
          <div
            key={detail.id}
            style={{
              border: '1px solid var(--semi-color-border)',
              borderRadius: 8,
              marginBottom: 16,
              padding: 16,
            }}
          >
            <div style={{ marginBottom: 12 }}>
              <Tag color="blue" size="small">第 {index + 1} 轮</Tag>
            </div>

            <div
              style={{
                background: 'var(--semi-color-primary-light-default)',
                borderRadius: 8,
                padding: '8px 12px',
                marginBottom: 12,
                maxWidth: '85%',
              }}
            >
              <Text type="secondary" style={{ fontSize: 12 }}>用户</Text>
              <div
                style={{
                  marginTop: 4,
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-word',
                  fontSize: 14,
                  lineHeight: 1.6,
                }}
              >
                {userMsg}
              </div>
            </div>

            <div
              style={{
                background: 'var(--semi-color-fill-0)',
                borderRadius: 8,
                padding: '8px 12px',
                marginBottom: 12,
                maxWidth: '85%',
                marginLeft: 'auto',
              }}
            >
              <Text type="secondary" style={{ fontSize: 12 }}>助手</Text>
              <div
                style={{
                  marginTop: 4,
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-word',
                  fontSize: 14,
                  lineHeight: 1.6,
                }}
              >
                {assistantMsg}
              </div>
            </div>

            <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', alignItems: 'center', marginBottom: 8 }}>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {formatDateTime(detail.timestamp)}
              </Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                <Tag size="small">{detail.model_original || '-'}</Tag>
                {' '}
                &rarr;
                {' '}
                <Tag size="small" color="blue">{detail.model_mapped || '-'}</Tag>
              </Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {formatDuration(detail.duration_ms)}
              </Text>
              <Tag
                color={(detail.status_code ?? 0) >= 400 ? 'red' : 'green'}
                size="small"
              >
                {detail.status_code ?? '-'}
              </Tag>
            </div>

            <Button
              size="small"
              type="tertiary"
              onClick={() => {
                onOpenRaw(
                  `第 ${index + 1} 轮原始内容`,
                  [
                    '=== 请求体 ===',
                    JSON.stringify(detail.request_body, null, 2),
                    '',
                    '=== 响应体 ===',
                    detail.response_body || '(空)',
                  ].join('\n'),
                );
              }}
            >
              展开原始内容
            </Button>
          </div>
        );
      })}
    </div>
  );
}
