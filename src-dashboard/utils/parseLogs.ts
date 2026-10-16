/**
 * Anthropic 响应体解析工具模块
 *
 * 把原始 `response_body` 解析为结构化 content blocks（thinking / text / tool_use），
 * 同时提供请求文本的 XML 标签清洗。支持 SSE 流式与 JSON 非流式两种格式，
 * 全部在客户端完成，不依赖后端预解析。
 *
 * OpenAI 协议族（Chat Completions / Responses）的响应解析见 `parseOpenAI.ts`。
 */

// ─── 类型 ───

/** 响应体格式类型 */
export type ResponseFormat = 'sse' | 'json';

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

// ─── XML 标签清洗 ───

/** 清洗请求消息中的 XML 标签 */
export function cleanXmlTags(text: string): string {
  return text
    .replace(/<session>/gi, '')
    .replace(/<\/session>/gi, '')
    .replace(/<system-reminder>/gi, '系统提醒：')
    .replace(/<\/system-reminder>/gi, '');
}

// ─── 格式检测 ───

/**
 * 通过 JSON.parse 试探自动判定响应体格式（内部辅助函数）
 *
 * 仅当调用方未显式传入 format 时使用。
 * SSE 格式（event:/data: 行）无法被 JSON.parse 成功解析，
 * JSON 格式则一定能被成功解析。
 */
function isJsonFormat(responseBody: string): boolean {
  try {
    JSON.parse(responseBody);
    return true;
  } catch {
    return false;
  }
}

/**
 * 从响应头中检测响应体格式
 *
 * 根据 Content-Type 响应头判定：
 * - `text/event-stream` → SSE 流式格式
 * - 其他（含 `application/json`、缺失等）→ JSON 格式
 *
 * @param responseHeaders - 响应头键值对（key 为小写）
 * @returns 响应体格式类型
 */
export function detectResponseFormat(
  responseHeaders: Record<string, unknown> | null | undefined,
): ResponseFormat {
  if (!responseHeaders) return 'json';

  const contentType = responseHeaders['content-type'];
  if (typeof contentType === 'string' && contentType.includes('text/event-stream')) {
    return 'sse';
  }

  return 'json';
}

// ─── JSON 响应体解析 ───

/**
 * 解析 JSON 格式响应体为结构化 content blocks
 *
 * JSON 格式的 content 数组中每个元素已是完整 block（含 type、text、name、id、input 等），
 * 不需要像 SSE 那样按 index 分组拼接 delta，直接映射为 ContentBlockInfo 即可。
 *
 * 示例响应：
 * ```json
 * {
 *   "id": "...",
 *   "type": "message",
 *   "model": "deepseek-v4-pro",
 *   "content": [
 *     {"type": "text", "text": "..."},
 *     {"type": "tool_use", "name": "...", "id": "...", "input": {...}}
 *   ],
 *   "stop_reason": "end_turn",
 *   "usage": {...}
 * }
 * ```
 *
 * 映射规则：
 * - content[i].type → ContentBlockInfo.block_type
 * - content[i].text → ContentBlockInfo.text
 * - content[i].thinking → ContentBlockInfo.thinking
 * - content[i].name → ContentBlockInfo.tool_name
 * - content[i].id → ContentBlockInfo.tool_use_id
 * - content[i].input → ContentBlockInfo.input
 * - 数组索引 → ContentBlockInfo.index
 */
function parseJsonResponse(responseBody: string): StructuredSSEResult {
  const json = JSON.parse(responseBody);
  const contentArray = Array.isArray(json.content)
    ? (json.content as Array<Record<string, unknown>>)
    : [];
  const result: StructuredSSEResult = {
    content_blocks: [],
  };

  // 提取 message_start 信息（模型、消息 ID、usage）
  result.message_start = {
    model: typeof json.model === 'string' ? json.model : undefined,
    message_id: typeof json.id === 'string' ? json.id : undefined,
    usage: json.usage as Record<string, unknown> | undefined,
  };

  // 映射 content 数组为 ContentBlockInfo[]
  result.content_blocks = contentArray.map(
    (block: Record<string, unknown>, idx: number): ContentBlockInfo => {
      const blockType = String(block.type ?? '');
      const mappedType =
        blockType === 'thinking' ? 'thinking' : blockType === 'tool_use' ? 'tool_use' : 'text'; // 'text' 和未知类型均按 text 处理

      const info: ContentBlockInfo = {
        index: idx,
        block_type: mappedType,
      };

      if (mappedType === 'text' && typeof block.text === 'string') {
        info.text = block.text;
      }
      if (mappedType === 'thinking') {
        if (typeof block.thinking === 'string') {
          info.thinking = block.thinking;
        }
        // 部分实现把 thinking 内容放在 text 字段，兼容处理
        if (typeof block.text === 'string' && !info.thinking) {
          info.thinking = block.text;
        }
      }
      if (mappedType === 'tool_use') {
        if (typeof block.name === 'string') {
          info.tool_name = block.name;
        }
        if (typeof block.id === 'string') {
          info.tool_use_id = block.id;
        }
        if (block.input && typeof block.input === 'object') {
          info.input = block.input as Record<string, unknown>;
        }
      }

      return info;
    },
  );

  // 提取 message_delta 信息（stop_reason、usage）
  const stopReason = json.stop_reason;
  if (typeof stopReason === 'string' || json.usage) {
    result.message_delta = {
      stop_reason: typeof stopReason === 'string' ? stopReason : undefined,
      usage: json.usage as Record<string, unknown> | undefined,
    };
  }

  return result;
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

// ─── 结构化 content block 解析 ───

/**
 * 将响应体字符串解析为结构化 content blocks
 *
 * 支持 SSE 流式格式和 JSON 非流式格式。
 * 格式检测由调用方通过 responseHeaders 的 Content-Type 完成，
 * 缺省时内部通过 JSON.parse 试探自动判定。
 *
 * SSE 路径：逐行解析 → 按 index 分组将 delta 拼接为完整 content block
 * JSON 路径：JSON.parse → 遍历 content 数组直接映射
 * → 输出统一的结构化 blocks 数组，同时提取 message_start / message_delta 信息。
 *
 * @param responseBody - 响应体原始字符串
 * @param format - 响应体格式（可选），缺省时自动检测
 */
export function parseStructuredBlocks(
  responseBody: string,
  format?: ResponseFormat,
): StructuredSSEResult {
  // 格式检测：显式传入 > Content-Type 判定 > JSON.parse 试探
  const resolvedFormat = format ?? (isJsonFormat(responseBody) ? 'json' : 'sse');

  if (resolvedFormat === 'json') {
    return parseJsonResponse(responseBody);
  }

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
          block.input = JSON.parse(trimmed + (trimmed.endsWith('}') ? '' : '}')) as Record<
            string,
            unknown
          >;
        } catch {
          // 无法解析，保留原始字符串
          block.input = { partial_json: fullJson };
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
