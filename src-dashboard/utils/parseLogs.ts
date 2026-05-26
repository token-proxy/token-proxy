/**
 * 日志解析工具模块
 *
 * 从原始 request_body / response_body 中提取和清洗文本，
 * 构建会话事件流 — 全部在客户端完成，不依赖后端预解析。
 */

// ─── 类型 ───

export interface ParsedContentBlock {
  block_type: 'text' | 'thinking' | 'tool_use' | 'redacted_thinking' | 'tool_result';
  content?: string;
  thinking_content?: string;
  signature?: string;
  tool_use_id?: string;
  tool_name?: string;
  input?: Record<string, unknown>;
}

export interface ConversationEvent {
  id: string;
  log_id: string;
  timestamp: string;
  request_index: number;
  event_index: number;
  source: string;
  role: 'user' | 'assistant';
  event_type: 'user_message' | 'assistant_message' | 'assistant_thinking' | 'tool_use' | 'agent_call';
  agent_id?: string;
  agent_type?: string;
  tool_use_id?: string;
  tool_name?: string;
  title?: string;
  content?: string;
  thinking_content?: string;
  display_payload?: Record<string, unknown>;
  tool_result_content?: string;
}

// ─── XML 标签清洗 ───

export function cleanXmlTags(text: string): string {
  return text
    .replace(/<session>/gi, '')
    .replace(/<\/session>/gi, '')
    .replace(/<system-reminder>/gi, '系统提醒：')
    .replace(/<\/system-reminder>/gi, '');
}

// ─── 请求体解析 ───

function extractContentText(content: unknown): string {
  if (typeof content === 'string') return content;

  if (Array.isArray(content)) {
    return (content as Array<Record<string, unknown>>)
      .map((block) => {
        if (block.type === 'text' && typeof block.text === 'string') return block.text;
        if (block.type === 'tool_result') {
          const inner = block.content;
          if (typeof inner === 'string') return inner;
          if (Array.isArray(inner)) return extractContentText(inner);
          return '';
        }
        return '';
      })
      .filter(Boolean)
      .join('');
  }

  return '';
}

export function extractLastUserMessage(
  requestBody: Record<string, unknown> | null | undefined,
): string | null {
  if (!requestBody) return null;

  const messages = (requestBody as Record<string, unknown>).messages;
  if (!Array.isArray(messages) || messages.length === 0) return null;

  // 从后向前找最后一条 role === 'user' 的消息
  for (let i = messages.length - 1; i >= 0; i--) {
    const m = messages[i] as Record<string, unknown>;
    if (m.role === 'user') {
      return cleanXmlTags(extractContentText(m.content));
    }
  }

  return null;
}

export function createMessagePreview(
  requestBody: Record<string, unknown> | null | undefined,
): string | null {
  const text = extractLastUserMessage(requestBody);
  if (!text) return null;
  // 压缩为单行，截取前 200 字符
  const singleLine = text.replace(/\n/g, ' ');
  return singleLine.length > 200 ? singleLine.slice(0, 200) + '...' : singleLine;
}

// ─── SSE 响应体解析 ───

interface SseEvent {
  kind: string;
  index?: number;
  data: Record<string, unknown>;
}

function parseSSE(responseBody: string): SseEvent[] {
  const events: SseEvent[] = [];
  const lines = responseBody.split('\n');
  let currentType = '';
  const dataLines: string[] = [];

  const flush = () => {
    if (currentType && dataLines.length > 0) {
      const jsonStr = dataLines.join('\n');
      try {
        const parsed = JSON.parse(jsonStr);
        events.push({
          kind: currentType,
          index: typeof parsed.index === 'number' ? parsed.index : undefined,
          data: parsed,
        });
      } catch {
        // 跳过无效 JSON
      }
    }
    currentType = '';
    dataLines.length = 0;
  };

  for (const raw of lines) {
    const line = raw.trim();
    if (line === '') {
      flush();
      continue;
    }
    if (line.startsWith('event:')) {
      currentType = line.slice(6).trim();
    } else if (line.startsWith('data:')) {
      const data = line.slice(5).trim();
      if (data !== '[DONE]') {
        dataLines.push(data);
      }
    }
  }
  flush();

  return events;
}

export function detectHasThinking(responseBody: string): boolean {
  const events = parseSSE(responseBody);
  return events.some((e) => {
    if (e.kind === 'content_block_start') {
      const block = e.data.content_block as Record<string, unknown> | undefined;
      return block?.type === 'thinking';
    }
    return false;
  });
}

export function detectHasToolUse(responseBody: string): boolean {
  const events = parseSSE(responseBody);
  return events.some((e) => {
    if (e.kind === 'content_block_start') {
      const block = e.data.content_block as Record<string, unknown> | undefined;
      return block?.type === 'tool_use';
    }
    return false;
  });
}

export function extractAssistantTextFromSSE(responseBody: string): string | null {
  const events = parseSSE(responseBody);
  const textParts: string[] = [];

  for (const ev of events) {
    if (ev.kind === 'content_block_delta') {
      const delta = ev.data.delta as Record<string, unknown> | undefined;
      if (delta?.type === 'text_delta' && typeof delta.text === 'string') {
        textParts.push(delta.text);
      }
    }
  }

  return textParts.length > 0 ? textParts.join('') : null;
}

export function extractThinkingFromSSE(responseBody: string): string | null {
  const events = parseSSE(responseBody);
  const parts: string[] = [];

  for (const ev of events) {
    if (ev.kind === 'content_block_delta') {
      const delta = ev.data.delta as Record<string, unknown> | undefined;
      if (
        delta?.type === 'thinking_delta' &&
        typeof delta.thinking === 'string'
      ) {
        parts.push(delta.thinking);
      }
    }
  }

  return parts.length > 0 ? parts.join('') : null;
}

export function extractAgentTypeFromSSE(responseBody: string): string | null {
  const events = parseSSE(responseBody);

  for (const ev of events) {
    if (ev.kind === 'content_block_start') {
      const block = ev.data.content_block as Record<string, unknown> | undefined;
      if (block?.type === 'tool_use' && block.name === 'Agent') {
        const input = ev.data.input || block.input;
        if (input && typeof input === 'object') {
          const subagentType = (input as Record<string, unknown>).subagent_type;
          if (typeof subagentType === 'string') return subagentType;
        }
      }
    }
  }

  return null;
}

// ─── 会话事件构建 ───

/** 每个 log 生成一个 crypto.randomUUID */
function uid(): string {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return Math.random().toString(36).slice(2) + Date.now().toString(36);
}

interface BuildEventsMeta {
  log_id: string;
  timestamp: string;
  request_index: number;
  agent_id?: string;
  agent_type?: string;
  conversation_source: string;
}

export function buildConversationEvents(
  requestBody: Record<string, unknown> | null | undefined,
  responseBody: string,
  meta: BuildEventsMeta,
): ConversationEvent[] {
  const events: ConversationEvent[] = [];
  let eventIndex = 0;

  // 1. 用户消息事件
  const userText = extractLastUserMessage(requestBody);
  if (userText) {
    events.push({
      id: uid(),
      log_id: meta.log_id,
      timestamp: meta.timestamp,
      request_index: meta.request_index,
      event_index: eventIndex++,
      source: meta.conversation_source,
      role: 'user',
      event_type: 'user_message',
      agent_id: meta.agent_id,
      agent_type: meta.agent_type,
      content: userText,
    });
  }

  // 2. 解析 SSE 构建 content blocks
  const sseEvents = parseSSE(responseBody);

  // 按 index 分组 content blocks
  const blocks = new Map<number, {
    type: string;
    textParts: string[];
    thinkingParts: string[];
    tool_use_id?: string;
    tool_name?: string;
    input?: Record<string, unknown>;
  }>();

  for (const ev of sseEvents) {
    const idx = ev.index ?? 0;

    if (ev.kind === 'content_block_start') {
      const block = ev.data.content_block as Record<string, unknown> | undefined;
      blocks.set(idx, {
        type: String(block?.type ?? ''),
        textParts: [],
        thinkingParts: [],
        tool_use_id: typeof block?.id === 'string' ? block.id : undefined,
        tool_name: typeof block?.name === 'string' ? block.name : undefined,
        input: (ev.data.input || block?.input) as Record<string, unknown> | undefined,
      });
    }

    if (ev.kind === 'content_block_delta') {
      const delta = ev.data.delta as Record<string, unknown> | undefined;
      const existing = blocks.get(idx);
      if (!existing) continue;

      if (delta?.type === 'text_delta' && typeof delta.text === 'string') {
        existing.textParts.push(delta.text);
      }
      if (delta?.type === 'thinking_delta' && typeof delta.thinking === 'string') {
        existing.thinkingParts.push(delta.thinking);
      }
      if (delta?.type === 'input_json_delta' && typeof delta.partial_json === 'string') {
        existing.input = existing.input || {};
        // 仅保存最后一段 partial_json 作为近似
        try {
          const parsed = JSON.parse(delta.partial_json);
          Object.assign(existing.input, parsed);
        } catch {
          // partial JSON，忽略
        }
      }
    }
  }

  // 3. 按 index 排序输出事件
  const sortedIndices = Array.from(blocks.keys()).sort((a, b) => a - b);
  for (const idx of sortedIndices) {
    const block = blocks.get(idx)!;

    if (block.thinkingParts.length > 0) {
      events.push({
        id: uid(),
        log_id: meta.log_id,
        timestamp: meta.timestamp,
        request_index: meta.request_index,
        event_index: eventIndex++,
        source: meta.conversation_source,
        role: 'assistant',
        event_type: 'assistant_thinking',
        agent_id: meta.agent_id,
        agent_type: meta.agent_type,
        thinking_content: block.thinkingParts.join(''),
      });
    }

    if (block.type === 'tool_use' && block.tool_name) {
      const isAgent = block.tool_name === 'Agent';
      events.push({
        id: uid(),
        log_id: meta.log_id,
        timestamp: meta.timestamp,
        request_index: meta.request_index,
        event_index: eventIndex++,
        source: meta.conversation_source,
        role: 'assistant',
        event_type: isAgent ? 'agent_call' : 'tool_use',
        agent_id: meta.agent_id,
        agent_type:
          meta.agent_type ||
          (isAgent && block.input
            ? String((block.input as Record<string, unknown>).subagent_type || '')
            : undefined),
        tool_use_id: block.tool_use_id,
        tool_name: block.tool_name,
        title: isAgent ? `Agent 调用: ${block.tool_name}` : `工具调用: ${block.tool_name}`,
        display_payload: block.input,
      });
    }

    if (block.textParts.length > 0) {
      events.push({
        id: uid(),
        log_id: meta.log_id,
        timestamp: meta.timestamp,
        request_index: meta.request_index,
        event_index: eventIndex++,
        source: meta.conversation_source,
        role: 'assistant',
        event_type: 'assistant_message',
        agent_id: meta.agent_id,
        agent_type: meta.agent_type,
        content: block.textParts.join(''),
      });
    }
  }

  return events;
}
