/**
 * OpenAI 协议响应/请求体解析工具。
 *
 * 与 parseLogs.ts 中的 Anthropic 解析器并列，
 * 由 ResponseContentCard / RequestContentCard 按 api_type 调用。
 */

import type { ContentBlockInfo } from './parseLogs.ts';

// ─── 类型 ───

/** OpenAI Chat Completions 消息 */
export interface OpenAIChatMessage {
  role: string;
  content: string | null;
  name?: string;
  tool_call_id?: string;
  tool_calls?: Array<{
    id: string;
    type: 'function';
    function: { name: string; arguments: string };
  }>;
}

/** OpenAI Chat Completions 工具定义 */
export interface OpenAIChatTool {
  type: 'function';
  function: {
    name: string;
    description?: string;
    parameters?: Record<string, unknown>;
  };
}

/** OpenAI Chat Completions 请求体解析结果 */
export interface OpenAIChatRequest {
  kind: 'chat';
  model: string;
  messages: OpenAIChatMessage[];
  tools?: OpenAIChatTool[];
  /** 其他请求参数（temperature、max_tokens 等） */
  params: Record<string, unknown>;
}

/** OpenAI Responses API 请求体解析结果 */
export interface OpenAIResponsesRequest {
  kind: 'responses';
  model: string;
  input: unknown[];
  instructions?: string;
  tools?: unknown[];
  params: Record<string, unknown>;
}

/** OpenAI 请求体解析联合类型 */
export type OpenAIParsedRequest = OpenAIChatRequest | OpenAIResponsesRequest;

// ─── Chat Completions 响应解析 ───

/**
 * 解析 OpenAI Chat Completions 的非流式响应体。
 *
 * 提取 choices[0].message.content 和 tool_calls 映射为 ContentBlockInfo[]。
 */
export function parseOpenAIChatResponse(body: string): ContentBlockInfo[] {
  try {
    const json = JSON.parse(body);
    const choice = json?.choices?.[0]?.message;
    if (!choice) return [];

    const blocks: ContentBlockInfo[] = [];
    let index = 0;

    // 1. 文本内容
    if (typeof choice.content === 'string' && choice.content.length > 0) {
      blocks.push({
        index: index++,
        block_type: 'text',
        text: choice.content,
      });
    }

    // 2. 工具调用
    const toolCalls = choice.tool_calls;
    if (Array.isArray(toolCalls)) {
      for (const tc of toolCalls) {
        const fn = tc?.function;
        if (fn?.name) {
          let input: Record<string, unknown> | undefined;
          if (typeof fn.arguments === 'string') {
            try {
              input = JSON.parse(fn.arguments);
            } catch {
              input = { raw_arguments: fn.arguments };
            }
          }
          blocks.push({
            index: index++,
            block_type: 'tool_use',
            tool_name: fn.name,
            tool_use_id: typeof tc.id === 'string' ? tc.id : undefined,
            input,
          });
        }
      }
    }

    return blocks;
  } catch {
    return [];
  }
}

/**
 * 解析 OpenAI Chat Completions 的 SSE 流式响应体。
 *
 * 累积每个 data chunk 的 choices[0].delta.content / delta.tool_calls。
 */
export function parseOpenAIChatSSE(body: string): ContentBlockInfo[] {
  const lines = body.split('\n');

  // 累积的文本内容
  let textContent = '';
  // 工具调用累积：Map<index, { id, name, arguments }>
  const toolCallsMap = new Map<number, { id: string; name: string; args: string }>();

  for (const raw of lines) {
    const line = raw.trim();
    if (!line.startsWith('data: ')) continue;

    const dataStr = line.slice(6);
    if (dataStr === '[DONE]') continue;

    try {
      const parsed = JSON.parse(dataStr);
      const delta = parsed?.choices?.[0]?.delta;
      if (!delta) continue;

      // 文本增量
      if (typeof delta.content === 'string') {
        textContent += delta.content;
      }

      // 工具调用增量
      const deltaToolCalls = delta.tool_calls;
      if (Array.isArray(deltaToolCalls)) {
        for (const tc of deltaToolCalls) {
          const idx = typeof tc.index === 'number' ? tc.index : 0;
          const existing = toolCallsMap.get(idx) || { id: '', name: '', args: '' };

          if (typeof tc.id === 'string') existing.id = tc.id;
          if (tc?.function?.name) existing.name += tc.function.name;
          if (typeof tc?.function?.arguments === 'string') existing.args += tc.function.arguments;

          toolCallsMap.set(idx, existing);
        }
      }
    } catch {
      // 跳过无效 JSON 行
    }
  }

  const blocks: ContentBlockInfo[] = [];
  let index = 0;

  // 文本块
  if (textContent.length > 0) {
    blocks.push({
      index: index++,
      block_type: 'text',
      text: textContent,
    });
  }

  // 工具调用块（按 index 排序）
  const sortedIndices = Array.from(toolCallsMap.keys()).sort((a, b) => a - b);
  for (const tcIdx of sortedIndices) {
    const tc = toolCallsMap.get(tcIdx)!;
    if (!tc.name) continue;

    let input: Record<string, unknown> | undefined;
    if (tc.args) {
      try {
        input = JSON.parse(tc.args);
      } catch {
        input = { partial_json: tc.args };
      }
    }

    blocks.push({
      index: index++,
      block_type: 'tool_use',
      tool_name: tc.name,
      tool_use_id: tc.id || undefined,
      input,
    });
  }

  return blocks;
}

// ─── Responses API 响应解析 ───

/**
 * 解析 OpenAI Responses API 的非流式响应体。
 *
 * 遍历 output[] 数组，按 type 分发：
 * - type: "message" → 其 content[] 中的 output_text 为文本块
 * - type: "reasoning" → 推理块
 * - type: "function_call" → 工具调用块
 * - type: "web_search_call" → 搜索调用块
 */
export function parseOpenAIResponsesResponse(body: string): ContentBlockInfo[] {
  try {
    const json = JSON.parse(body);
    const output = json?.output;
    if (!Array.isArray(output)) return [];

    const blocks: ContentBlockInfo[] = [];
    let index = 0;

    for (const item of output) {
      const itemType = String(item?.type ?? '');

      switch (itemType) {
        case 'message': {
          // 提取 content[] 中的 output_text 和 tool_call
          const content = item.content;
          if (Array.isArray(content)) {
            for (const contentItem of content) {
              const cType = String(contentItem?.type ?? '');
              if (cType === 'output_text' && typeof contentItem.text === 'string') {
                blocks.push({
                  index: index++,
                  block_type: 'text',
                  text: contentItem.text,
                });
              }
            }
          }
          break;
        }

        case 'reasoning': {
          // 推理内容映射为 thinking 块
          const summary = item.summary;
          if (Array.isArray(summary)) {
            const textParts = summary
              .filter(
                (s: Record<string, unknown>) =>
                  s.type === 'summary_text' && typeof s.text === 'string',
              )
              .map((s: Record<string, unknown>) => s.text as string);
            if (textParts.length > 0) {
              blocks.push({
                index: index++,
                block_type: 'thinking',
                thinking: textParts.join('\n'),
              });
            }
          }
          // 某些响应中 reasoning 的 content 直接在 item.content 中
          if (typeof item.content === 'string') {
            blocks.push({
              index: index++,
              block_type: 'thinking',
              thinking: item.content,
            });
          }
          break;
        }

        case 'function_call': {
          blocks.push({
            index: index++,
            block_type: 'tool_use',
            tool_name: typeof item.name === 'string' ? item.name : undefined,
            tool_use_id: typeof item.call_id === 'string' ? item.call_id : undefined,
            input: parseToolInput(item.arguments),
          });
          break;
        }

        case 'web_search_call': {
          // 搜索调用视为特殊的工具调用
          blocks.push({
            index: index++,
            block_type: 'tool_use',
            tool_name: 'web_search',
            tool_use_id: typeof item.id === 'string' ? item.id : undefined,
          });
          break;
        }

        default:
          // 未知类型，跳过
          break;
      }
    }

    return blocks;
  } catch {
    return [];
  }
}

/**
 * 解析 OpenAI Responses API 的 SSE 流式响应体。
 *
 * 按事件 type 累积：
 * - response.output_text.delta → 累积文本
 * - response.reasoning_text.delta → 累积推理
 * - response.function_call_arguments.delta → 累积工具调用参数
 * - response.output_item.done → 触发块边界
 */
export function parseOpenAIResponsesSSE(body: string): ContentBlockInfo[] {
  const lines = body.split('\n');

  const blocks: ContentBlockInfo[] = [];
  let currentBlock: ContentBlockInfo | null = null;
  let index = 0;

  let i = 0;
  while (i < lines.length) {
    // 收集 SSE 事件行
    let eventType = '';
    const dataLines: string[] = [];

    while (i < lines.length && lines[i].trim() !== '') {
      const trimmed = lines[i].trim();
      if (trimmed.startsWith('event: ')) {
        eventType = trimmed.slice(7);
      } else if (trimmed.startsWith('data: ')) {
        const data = trimmed.slice(6);
        if (data !== '[DONE]') {
          dataLines.push(data);
        }
      }
      i++;
    }
    // 跳过空行
    i++;

    if (dataLines.length === 0) continue;

    try {
      const parsed = JSON.parse(dataLines.join('\n'));

      switch (eventType) {
        case 'response.output_text.delta': {
          const delta = typeof parsed.delta === 'string' ? parsed.delta : '';
          if (!currentBlock || currentBlock.block_type !== 'text') {
            // 结束上一个块
            if (currentBlock) {
              blocks.push(currentBlock);
            }
            currentBlock = {
              index: index++,
              block_type: 'text',
              text: delta,
            };
          } else {
            currentBlock.text = (currentBlock.text || '') + delta;
          }
          break;
        }

        case 'response.reasoning_text.delta': {
          const delta = typeof parsed.delta === 'string' ? parsed.delta : '';
          if (!currentBlock || currentBlock.block_type !== 'thinking') {
            if (currentBlock) {
              blocks.push(currentBlock);
            }
            currentBlock = {
              index: index++,
              block_type: 'thinking',
              thinking: delta,
            };
          } else {
            currentBlock.thinking = (currentBlock.thinking || '') + delta;
          }
          break;
        }

        case 'response.function_call_arguments.delta': {
          const delta = typeof parsed.delta === 'string' ? parsed.delta : '';
          if (!currentBlock || currentBlock.block_type !== 'tool_use') {
            if (currentBlock) {
              blocks.push(currentBlock);
            }
            currentBlock = {
              index: index++,
              block_type: 'tool_use',
              input: {},
              _argsBuffer: delta,
            } as ContentBlockInfo & { _argsBuffer?: string };
          } else {
            (currentBlock as ContentBlockInfo & { _argsBuffer: string })._argsBuffer =
              ((currentBlock as ContentBlockInfo & { _argsBuffer?: string })._argsBuffer || '') +
              delta;
          }
          break;
        }

        case 'response.output_item.done': {
          // 块边界：完善当前块的信息
          if (currentBlock) {
            const item = parsed.item;
            if (item) {
              if (item.type === 'function_call' || item.type === 'web_search_call') {
                currentBlock.block_type = 'tool_use';
                currentBlock.tool_name = item.name || currentBlock.tool_name || 'web_search';
                currentBlock.tool_use_id = item.call_id || item.id || currentBlock.tool_use_id;
              }
              // 尝试解析累积的 JSON 参数
              if (item.type === 'function_call') {
                const argsStr = item.arguments;
                if (typeof argsStr === 'string') {
                  try {
                    currentBlock.input = JSON.parse(argsStr);
                  } catch {
                    // 保留原始参数
                  }
                }
              }
            }
            blocks.push(currentBlock);
            currentBlock = null;
          }
          break;
        }

        default:
          break;
      }
    } catch {
      // 跳过无效 JSON
    }
  }

  // 处理未结束的最后一个块
  if (currentBlock) {
    // 尝试解析 _argsBuffer
    const buf = (currentBlock as ContentBlockInfo & { _argsBuffer?: string })._argsBuffer;
    if (buf && currentBlock.block_type === 'tool_use') {
      try {
        currentBlock.input = JSON.parse(buf);
      } catch {
        currentBlock.input = { partial_json: buf };
      }
    }
    // 清理内部字段
    delete (currentBlock as unknown as Record<string, unknown>)._argsBuffer;
    blocks.push(currentBlock);
  }

  return blocks;
}

// ─── 请求体解析 ───

/**
 * 解析 OpenAI 请求体为可展示的结构化数据。
 *
 * 自动检测 Chat Completions 和 Responses API 两种格式，
 * 返回联合类型 OpenAIParsedRequest。
 */
export function parseOpenAIRequestBody(
  body: Record<string, unknown> | string,
): OpenAIParsedRequest | null {
  let parsed: Record<string, unknown>;
  if (typeof body === 'string') {
    try {
      parsed = JSON.parse(body);
    } catch {
      return null;
    }
  } else {
    parsed = body;
  }

  // 检测格式：Responses API 有 input 字段
  if (parsed.input !== undefined) {
    return parseResponsesRequestBody(parsed);
  }

  // 默认为 Chat Completions
  return parseChatRequestBody(parsed);
}

/**
 * 解析 Chat Completions 请求体。
 */
function parseChatRequestBody(body: Record<string, unknown>): OpenAIChatRequest {
  const messages: OpenAIChatMessage[] = [];

  if (Array.isArray(body.messages)) {
    for (const msg of body.messages as Array<Record<string, unknown>>) {
      const openaiMsg: OpenAIChatMessage = {
        role: String(msg.role ?? ''),
        content: typeof msg.content === 'string' ? msg.content : null,
      };

      if (typeof msg.name === 'string') openaiMsg.name = msg.name;
      if (typeof msg.tool_call_id === 'string') openaiMsg.tool_call_id = msg.tool_call_id;

      const toolCalls = msg.tool_calls;
      if (Array.isArray(toolCalls)) {
        openaiMsg.tool_calls = toolCalls.map((tc: Record<string, unknown>) => ({
          id: String(tc.id ?? ''),
          type: 'function' as const,
          function: {
            name: String((tc.function as Record<string, unknown>)?.name ?? ''),
            arguments: String((tc.function as Record<string, unknown>)?.arguments ?? ''),
          },
        }));
      }

      messages.push(openaiMsg);
    }
  }

  const tools: OpenAIChatTool[] = [];
  if (Array.isArray(body.tools)) {
    for (const tool of body.tools as Array<Record<string, unknown>>) {
      tools.push({
        type: 'function',
        function: {
          name: String((tool.function as Record<string, unknown>)?.name ?? ''),
          description:
            typeof (tool.function as Record<string, unknown>)?.description === 'string'
              ? ((tool.function as Record<string, unknown>).description as string)
              : undefined,
          parameters: (tool.function as Record<string, unknown>)?.parameters as
            | Record<string, unknown>
            | undefined,
        },
      });
    }
  }

  return {
    kind: 'chat',
    model: String(body.model ?? ''),
    messages,
    tools: tools.length > 0 ? tools : undefined,
    params: body,
  };
}

/**
 * 解析 Responses API 请求体。
 */
function parseResponsesRequestBody(body: Record<string, unknown>): OpenAIResponsesRequest {
  return {
    kind: 'responses',
    model: String(body.model ?? ''),
    input: Array.isArray(body.input) ? (body.input as unknown[]) : [],
    instructions: typeof body.instructions === 'string' ? body.instructions : undefined,
    tools: Array.isArray(body.tools) ? (body.tools as unknown[]) : undefined,
    params: body,
  };
}

// ─── 辅助函数 ───

/** 解析工具调用的 arguments（可能是 JSON 字符串或对象） */
function parseToolInput(args: unknown): Record<string, unknown> | undefined {
  if (!args) return undefined;
  if (typeof args === 'object' && args !== null && !Array.isArray(args)) {
    return args as Record<string, unknown>;
  }
  if (typeof args === 'string') {
    try {
      return JSON.parse(args);
    } catch {
      return { raw_arguments: args };
    }
  }
  return undefined;
}
