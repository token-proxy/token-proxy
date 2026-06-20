/**
 * 日志解析工具模块
 *
 * 从原始 request_body / response_body 中提取和清洗文本，
 * 构建会话事件流 — 全部在客户端完成，不依赖后端预解析。
 */

import type { ConversationTurn, SessionContentItem, TokenUsage, TurnBlock, TurnTokenSummary } from '../types/log';

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

/** 清洗请求消息中的 XML 标签 */
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

/** 提取请求体中最后一条用户消息的文本 */
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

// ─── 轮次分组辅助函数 ───

/** tool_result 信息 */
interface ToolResultInfo {
  toolUseId: string;
  content: string;
  isError: boolean;
}

/**
 * 提取 messages 中最新一条用户消息的非 tool_result 文本
 *
 * 遍历 messages，从后向前找第一条 role === 'user' 的消息，
 * 提取其中非 tool_result 内容块的文本。
 *
 * @returns 用户文本字符串，若无则返回 null
 */
function extractUserMessage(messages: unknown[]): string | null {
  // 1. 从后向前找最后一条 role === 'user' 的消息
  for (let i = messages.length - 1; i >= 0; i--) {
    const m = messages[i] as Record<string, unknown>;
    if (m.role !== 'user') continue;

    const content = m.content;
    if (!content) continue;

    // 2. 提取非 tool_result 的文本块
    if (Array.isArray(content)) {
      const texts: string[] = [];
      for (const block of content as Array<Record<string, unknown>>) {
        if (block.type === 'text' && typeof block.text === 'string') {
          texts.push(block.text);
        }
      }
      if (texts.length > 0) return cleanXmlTags(texts.join(''));
    }

    // 3. content 可能是纯字符串（非数组格式）
    if (typeof content === 'string') return cleanXmlTags(content);
  }

  return null;
}

/**
 * 提取 messages 中所有 tool_result 内容块
 *
 * 遍历 messages 中的每条 user 消息，检查其 content 中的每个 block，
 * 提取 type === 'tool_result' 的内容。
 */
function extractToolResults(messages: unknown[]): ToolResultInfo[] {
  const results: ToolResultInfo[] = [];

  for (const msg of messages as Array<Record<string, unknown>>) {
    if (msg.role !== 'user') continue;

    const content = msg.content;
    if (!Array.isArray(content)) continue;

    for (const block of content as Array<Record<string, unknown>>) {
      if (block.type !== 'tool_result') continue;

      // 提取 tool_use_id
      const toolUseId = typeof block.tool_use_id === 'string' ? block.tool_use_id : '';

      // 提取 content（可能是字符串或数组）
      const blockContent = block.content;
      let contentStr = '';
      if (typeof blockContent === 'string') {
        contentStr = blockContent;
      } else if (Array.isArray(blockContent)) {
        contentStr = extractContentText(blockContent);
      }

      const isError = block.is_error === true || block.isError === true;

      results.push({ toolUseId, content: contentStr, isError });
    }
  }

  return results;
}

/**
 * 判断消息数组是否需要开启新轮次
 *
 * 判定依据：最新一条 user 消息的 content 中是否包含非 tool_result 的内容块。
 * 如果只包含 tool_result，说明是工具执行结果的提交，属于同一轮次的延续。
 */
function isNewTurnStart(messages: unknown[]): boolean {
  // 1. 从后向前找最后一条 role === 'user' 的消息
  for (let i = (messages as Array<unknown>).length - 1; i >= 0; i--) {
    const m = messages[i] as Record<string, unknown>;
    if (m.role !== 'user') continue;

    const content = m.content;
    if (!content) return false;

    // 2. 检查是否包含非 tool_result 的内容块
    if (Array.isArray(content)) {
      for (const block of content as Array<Record<string, unknown>>) {
        if (block.type !== 'tool_result' && block.type !== 'tool_use') {
          return true;
        }
      }
      return false; // 全部是 tool_result / tool_use，属于同一轮次延续
    }

    // 3. content 是字符串（非数组），说明是纯用户输入 → 新轮次
    return true;
  }

  return false;
}

/**
 * 创建消息预览文本
 *
 * 提取最后一条用户消息，压缩为单行，截取前 200 字符。
 */
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

/** 检测 SSE 响应中是否包含 thinking 块 */
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

/** 检测 SSE 响应中是否包含 tool_use 块 */
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

/** 从 SSE 响应中提取助手回复文本（拼接所有 text_delta） */
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

/** 从 SSE 响应中提取思考内容（拼接所有 thinking_delta） */
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

/** 从 SSE 响应中提取子代理类型（检测 tool_use name=Agent） */
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

// ─── 结构化 content block 解析 ───

export interface ContentBlockInfo {
  index: number;
  block_type: 'thinking' | 'text' | 'tool_use';
  /** thinking block 的 thinking 内容 */
  thinking?: string;
  /** text block 的文本内容 */
  text?: string;
  /** tool_use block 的工具名 */
  tool_name?: string;
  /** tool_use block 的调用 ID */
  tool_use_id?: string;
  /** tool_use block 的输入参数 JSON */
  input?: Record<string, unknown>;
}

export interface MessageStartInfo {
  model?: string;
  message_id?: string;
  usage?: Record<string, unknown>;
}

export interface MessageDeltaInfo {
  stop_reason?: string;
  usage?: Record<string, unknown>;
}

export interface StructuredSSEResult {
  /** message_start 事件提取的信息 */
  message_start?: MessageStartInfo;
  /** 按 index 排序的 content blocks */
  content_blocks: ContentBlockInfo[];
  /** message_delta 事件提取的信息 */
  message_delta?: MessageDeltaInfo;
}

/**
 * 将 SSE 响应体字符串解析为结构化 content blocks
 *
 * 逐行解析 SSE → 按 index 分组将 delta 拼接为完整 content block
 * → 输出结构化 blocks 数组，同时提取 message_start / message_delta 信息。
 */
export function parseStructuredBlocks(responseBody: string): StructuredSSEResult {
  const events = parseSSE(responseBody);
  const result: StructuredSSEResult = {
    content_blocks: [],
  };

  // 1. 提取 message_start
  for (const ev of events) {
    if (ev.kind === 'message_start') {
      const msg = ev.data.message as Record<string, unknown> | undefined;
      result.message_start = {
        model: typeof msg?.model === 'string' ? msg.model : undefined,
        message_id: typeof msg?.id === 'string' ? msg.id : undefined,
        usage: msg?.usage as Record<string, unknown> | undefined,
      };
      break;
    }
  }

  // 2. 按 index 分组 content blocks
  const blocks = new Map<number, ContentBlockInfo>();
  const textParts = new Map<number, string[]>();
  const thinkingParts = new Map<number, string[]>();
  const inputJsonParts = new Map<number, string[]>();

  for (const ev of events) {
    const idx = ev.index ?? 0;

    if (ev.kind === 'content_block_start') {
      const block = ev.data.content_block as Record<string, unknown> | undefined;
      const blockType = String(block?.type ?? '');
      if (blockType === 'thinking' || blockType === 'text' || blockType === 'tool_use') {
        blocks.set(idx, {
          index: idx,
          block_type: blockType,
          tool_name: typeof block?.name === 'string' ? block.name : undefined,
          tool_use_id: typeof block?.id === 'string' ? block.id : undefined,
        });
      }
      if (!textParts.has(idx)) textParts.set(idx, []);
      if (!thinkingParts.has(idx)) thinkingParts.set(idx, []);
      if (!inputJsonParts.has(idx)) inputJsonParts.set(idx, []);
    }

    if (ev.kind === 'content_block_delta') {
      const delta = ev.data.delta as Record<string, unknown> | undefined;
      if (!textParts.has(idx)) textParts.set(idx, []);
      if (!thinkingParts.has(idx)) thinkingParts.set(idx, []);
      if (!inputJsonParts.has(idx)) inputJsonParts.set(idx, []);

      if (delta?.type === 'text_delta' && typeof delta.text === 'string') {
        textParts.get(idx)!.push(delta.text);
      }
      if (delta?.type === 'thinking_delta' && typeof delta.thinking === 'string') {
        thinkingParts.get(idx)!.push(delta.thinking);
      }
      if (delta?.type === 'input_json_delta' && typeof delta.partial_json === 'string') {
        inputJsonParts.get(idx)!.push(delta.partial_json);
      }
    }
  }

  // 3. 合并 delta 到 blocks
  for (const [idx, block] of blocks) {
    const textJoined = textParts.get(idx)?.join('') ?? '';
    const thinkingJoined = thinkingParts.get(idx)?.join('') ?? '';
    const inputParts = inputJsonParts.get(idx);

    if (textJoined) block.text = textJoined;
    if (thinkingJoined) block.thinking = thinkingJoined;

    if (block.block_type === 'tool_use' && inputParts && inputParts.length > 0) {
      const fullJson = inputParts.join('');
      try {
        block.input = JSON.parse(fullJson) as Record<string, unknown>;
      } catch {
        // 部分 JSON，用对象模式尝试近似解析
        try {
          // 掉尾部的截断字符
          const trimmed = fullJson.replace(/,\s*$/, '').replace(/[^}]\s*$/, '');
          block.input = JSON.parse(trimmed + (trimmed.endsWith('}') ? '' : '}')) as Record<string, unknown>;
        } catch {
          // 无法解析，保留原始字符串
          block.input = {partial_json: fullJson};
        }
      }
    }
  }

  // 4. 按 index 排序输出
  const sortedIndices = Array.from(blocks.keys()).sort((a, b) => a - b);
  result.content_blocks = sortedIndices.map((idx) => blocks.get(idx)!);

  // 5. 提取 message_delta
  for (const ev of events) {
    if (ev.kind === 'message_delta') {
      const delta = ev.data.delta as Record<string, unknown> | undefined;
      result.message_delta = {
        stop_reason: typeof delta?.stop_reason === 'string' ? delta.stop_reason : undefined,
        usage: ev.data.usage as Record<string, unknown> | undefined,
      };
      break;
    }
  }

  return result;
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
  agent_id?: string;
  agent_type?: string;
  conversation_source: string;
}

/**
 * 构建会话事件数组
 *
 * 从请求体和 SSE 响应体中提取用户消息和助手消息（含 thinking、tool_use、agent_call），
 * 按 event_index 排序输出。全部在客户端完成，不依赖后端预解析。
 */
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

// ─── 轮次构建 ───

/**
 * 构建会话的对话轮次列表
 *
 * 从所有 SessionContentItem 中提取对话内容，
 * 按轮次分组事件块，提取 tool_result，聚合 Token 用量。
 * 子代理调用通过 agent_call 的 children 字段递归嵌套。
 *
 * @param contents - 会话的所有原始日志内容（按 timestamp 排序）
 * @param tokenUsageMap - log_id 到 TokenUsage 的映射
 * @param maxAgentDepth - 子代理最大嵌套深度，默认 5
 * @returns 按时间排序的对话轮次列表
 */
export function buildConversationTurns(
  contents: SessionContentItem[],
  tokenUsageMap: Record<string, TokenUsage>,
  maxAgentDepth: number = 5,
): ConversationTurn[] {
  // 1. 按 timestamp 排序并预处理
  const sorted = [...contents].sort((a, b) =>
    a.timestamp.localeCompare(b.timestamp),
  );
  const preprocessed = sorted.map((item) => ({
    item,
    structured: parseStructuredBlocks(item.response_body),
  }));

  // 2. 遍历 contents 构建轮次
  const turns: ConversationTurn[] = [];
  let currentTurn: {
    items: typeof preprocessed;
    userMessage: string;
    logIds: string[];
  } | null = null;

  for (const entry of preprocessed) {
    const { item } = entry;
    const messages = item.request_body.messages as unknown[];

    // 跳过无 messages 的条目
    if (!messages || !Array.isArray(messages) || messages.length === 0) {
      continue;
    }

    const isNew = currentTurn === null || isNewTurnStart(messages);

    if (isNew) {
      // 将上一个轮次写入结果
      if (currentTurn !== null) {
        turns.push(buildTurn(currentTurn, tokenUsageMap, maxAgentDepth));
      }

      // 开启新轮次
      const userMsg = extractUserMessage(messages) || '';
      currentTurn = {
        items: [entry],
        userMessage: userMsg,
        logIds: [item.log_id],
      };
    } else if (currentTurn !== null) {
      // 属于当前轮次
      currentTurn.items.push(entry);
      currentTurn.logIds.push(item.log_id);
    }
  }

  // 3. 处理最后一个轮次
  if (currentTurn !== null) {
    turns.push(buildTurn(currentTurn, tokenUsageMap, maxAgentDepth));
  }

  // 4. 填充 turnIndex
  return turns.map((turn, idx) => ({
    ...turn,
    turnIndex: idx + 1,
  }));
}

/**
 * 从轮次原始数据构建 ConversationTurn
 *
 * 处理 tool_result 提取、SSE 事件块转换、子代理递归嵌套。
 */
function buildTurn(
  turnData: {
    items: Array<{ item: SessionContentItem; structured: StructuredSSEResult }>;
    userMessage: string;
    logIds: string[];
  },
  tokenUsageMap: Record<string, TokenUsage>,
  maxAgentDepth: number,
  currentDepth: number = 0,
): ConversationTurn {
  const { items, userMessage, logIds } = turnData;

  // 1. 构建 blocks
  const blocks: TurnBlock[] = [];

  for (const entry of items) {
    const { item, structured } = entry;

    // 1a. 提取 tool_result（来自请求体的 user 消息）
    const messages = item.request_body.messages as unknown[];
    if (Array.isArray(messages)) {
      const toolResults = extractToolResults(messages);
      for (const tr of toolResults) {
        blocks.push({
          type: 'tool_result',
          toolUseId: tr.toolUseId,
          content: tr.content,
          isError: tr.isError,
          logId: item.log_id,
          timestamp: item.timestamp,
        });
      }
    }

    // 1b. 解析响应体的 content blocks
    const contentBlocks = structured.content_blocks;
    for (const block of contentBlocks) {
      switch (block.block_type) {
        case 'thinking':
          if (block.thinking) {
            blocks.push({
              type: 'thinking',
              content: block.thinking,
              logId: item.log_id,
              timestamp: item.timestamp,
            });
          }
          break;

        case 'text':
          if (block.text) {
            blocks.push({
              type: 'assistant_message',
              content: block.text,
              logId: item.log_id,
              timestamp: item.timestamp,
            });
          }
          break;

        case 'tool_use':
          if (block.tool_name === 'Agent') {
            // 子代理调用块（由外层在子代理处理阶段填充 children）
            const agentType =
              String(
                (block.input as Record<string, unknown> | undefined)
                  ?.subagent_type || '',
              ) || '';
            blocks.push(buildAgentCallBlock(
              agentType,
              item.log_id,
              item.timestamp,
              [],
              maxAgentDepth,
              currentDepth,
            ));
          } else {
            blocks.push({
              type: 'tool_use',
              toolName: block.tool_name || '',
              input: block.input || {},
              logId: item.log_id,
              timestamp: item.timestamp,
            });
          }
          break;
      }
    }
  }

  // 2. 聚合 Token 用量
  const tokenSummary = aggregateTokens(logIds, tokenUsageMap);

  // 3. 构建时间范围
  const startTime = items[0] ? items[0].item.timestamp : logIds[0] || '';
  const lastItem = items[items.length - 1];
  const endTime = lastItem ? lastItem.item.timestamp : startTime;

  return {
    id: uid(),
    turnIndex: 0, // 由外层填充
    startTime,
    endTime,
    userMessage,
    blocks,
    tokenSummary,
    logIds: [...logIds],
  };
}

/**
 * 构建子代理调用块
 *
 * 子代理的 children 由外层在完成所有轮次分组后填充。
 * 超过 maxAgentDepth 时截断，返回空的 children 数组。
 */
function buildAgentCallBlock(
  agentType: string,
  logId: string,
  timestamp: string,
  children: TurnBlock[],
  maxAgentDepth: number,
  currentDepth: number,
): TurnBlock {
  if (currentDepth >= maxAgentDepth) {
    return {
      type: 'agent_call',
      agentType,
      children: [],
      logIds: [logId],
      tokenSummary: {
        inputTokens: 0,
        outputTokens: 0,
        cacheCreationTokens: 0,
        cacheReadTokens: 0,
        thinkingTokens: 0,
        totalTokens: 0,
      },
      logId,
      timestamp,
    };
  }

  return {
    type: 'agent_call',
    agentType,
    children,
    logIds: [logId],
    tokenSummary: {
      inputTokens: 0,
      outputTokens: 0,
      cacheCreationTokens: 0,
      cacheReadTokens: 0,
      thinkingTokens: 0,
      totalTokens: 0,
    },
    logId,
    timestamp,
  };
}

/**
 * 聚合多个 log_id 的 Token 用量
 */
function aggregateTokens(
  logIds: string[],
  tokenUsageMap: Record<string, TokenUsage>,
): TurnTokenSummary {
  const summary: TurnTokenSummary = {
    inputTokens: 0,
    outputTokens: 0,
    cacheCreationTokens: 0,
    cacheReadTokens: 0,
    thinkingTokens: 0,
    totalTokens: 0,
  };

  for (const lid of logIds) {
    const usage = tokenUsageMap[lid];
    if (!usage) continue;

    summary.inputTokens += usage.input_tokens || 0;
    summary.outputTokens += usage.output_tokens || 0;
    summary.cacheCreationTokens += usage.cache_creation_input_tokens || 0;
    summary.cacheReadTokens += usage.cache_read_input_tokens || 0;
    summary.thinkingTokens += usage.thinking_tokens || 0;
    summary.totalTokens += usage.total_tokens || 0;
  }

  return summary;
}
