/**
 * 请求日志 → 运行轨迹模型构建器
 *
 * 参照 DSH 轨迹视图的契约，把一条日志的请求体与响应体还原为「轮次 → 分组 → 记录行」三级账本：
 *
 * ```text
 * 第 N 轮                    ← 轮次（sticky 表头，含触发消息摘要与步数）
 *   消息                     ← 分组：系统提示 / 用户消息 / 助手消息
 *     #1 用户 …              ← 记录行：时间 · 类型 · 内容 · 模型 · 工具 · 输入 · 输出 · 思考
 *   第 k 步                   ← 分组：一次模型请求及其工具调用
 *     #2 请求 #1 …
 *     #3 助手 …（仅工具调用）
 *     #4 工具 bash {"…"}      ← 工具调用与结果配对在同一行
 * ```
 *
 * 同时产出三泳道时间轴（输入 / 模型 / 工具）与轮次边界。日志只记录**整体耗时**、
 * 不记录逐条耗时，因此时间轴按「序列」（等宽）投影、以记录顺序为轴，
 * 不做时间维度的虚构；轮次边界用于标注对话结构。
 *
 * 请求头与响应头分别挂在「请求」「响应」记录行的检查器页签里，不再单独占页面卡片。
 */

import type { LogDetailFull } from '../types/log';
import type { ContentBlockInfo } from './parseLogs.ts';
import { detectResponseFormat } from './parseLogs.ts';
import { parseResponseBlocks } from './responseBlocks.ts';
import type { ApiProtocolName } from './responseBlocks.ts';

// ─── 类型 ───

/** 记录行类型（对齐 DSH 的 TrajectoryCellKind 语义） */
export type TrajectoryCellKind =
  'request' | 'response' | 'system' | 'user' | 'assistant' | 'tool' | 'raw';

/** 时间轴泳道：输入 / 模型 / 工具 */
export type TrajectoryLane = 'input' | 'model' | 'tool';

/** 记录状态 */
export type TrajectoryStatus = 'completed' | 'failed' | 'pending';

/** 检查器页签键 */
export type TrajectoryTabKey =
  | 'overview'
  | 'content'
  | 'output'
  | 'thinking'
  | 'payload'
  | 'result'
  | 'headers'
  | 'options'
  | 'raw';

/** 检查器页签内容形态 */
export type TrajectoryTabKind = 'meta' | 'json' | 'text';

/** 键值元信息 */
export interface TrajectoryMetaItem {
  label: string;
  value: string;
  /** 长值（JSON、头部值等）用等宽字体展示 */
  mono?: boolean;
}

/** 检查器页签 */
export interface TrajectoryTab {
  key: TrajectoryTabKey;
  label: string;
  kind: TrajectoryTabKind;
  /** kind = meta 时的键值列表 */
  items?: TrajectoryMetaItem[];
  /** kind = json / text 时的内容 */
  value?: string;
}

/** 一条记录行 */
export interface TrajectoryCell {
  /** 全局 1 起的记录序号（显示 #N） */
  index: number;
  kind: TrajectoryCellKind;
  /** 类型标签：请求 / 响应 / 系统 / 用户 / 助手 / 工具 */
  kindLabel: string;
  /** 单行摘要（工具行为 `name {args}`，消息行为首行摘要） */
  text: string;
  /** 次级预览（工具结果摘要、消息正文续行等） */
  preview?: string;
  /** 模型列 */
  model?: string;
  /** 工具名（列与检查器共用） */
  toolName?: string;
  /** 调用 ID（工具行与消息块配对用） */
  callId?: string;
  /** 记录时间（仅本次请求的请求/响应行有真实时间） */
  time?: string;
  /** 自身耗时（毫秒，未记录为 null） */
  timeMs: number | null;
  /** 词元列（仅承载本次请求的响应行） */
  input?: number;
  cacheRead?: number;
  cacheWrite?: number;
  output?: number;
  think?: number;
  /** 状态 */
  status: TrajectoryStatus;
  isError: boolean;
  /** 该行对应的原文未落库（内容缺失，不是解析失败） */
  contentMissing?: boolean;
  /** 时间轴泳道（null = 不进时间轴） */
  lane: TrajectoryLane | null;
  /** 检查器页签 */
  tabs: TrajectoryTab[];
}

/** 分组：消息 / 第 k 步 */
export interface TrajectoryGroup {
  key: string;
  title: string;
  description?: string;
  cells: TrajectoryCell[];
}

/** 轮次 */
export interface TrajectoryTurn {
  index: number;
  title: string;
  /** 触发本轮的用户消息摘要 */
  description?: string;
  groups: TrajectoryGroup[];
}

/** 时间轴投影片段 */
/**
 * 时间轴片段
 *
 * 对齐 DSH 的 sequence 投影：**每条记录恰好占一个等宽槽位**（`slot`），
 * 因此片段之间在结构上不可能重叠 —— 泳道只决定纵向行，槽位决定横向位置。
 * 日志未记录逐条耗时，故不按真实时间布轴，也不做内容体量加权
 * （体量差异可达三个数量级，会出现一条极宽一条极窄）。
 */
export interface TrajectoryTimelineSpan {
  cellIndex: number;
  lane: TrajectoryLane;
  /** 槽位序号（0-based，等于该片段在时间轴上的横向位置） */
  slot: number;
  kind: TrajectoryCellKind;
  label: string;
  turnIndex: number | null;
}

/** 轮次边界（该轮次首条记录所在的槽位） */
export interface TrajectoryTimelineBoundary {
  turnIndex: number;
  slot: number;
}

/** 时间轴模型 */
export interface TrajectoryTimeline {
  spans: TrajectoryTimelineSpan[];
  boundaries: TrajectoryTimelineBoundary[];
  /** 记录总数（= 槽位数，等宽） */
  total: number;
}

/** 概览统计 */
export interface TrajectoryStats {
  turns: number;
  steps: number;
  cells: number;
  messages: number;
  toolCalls: number;
}

/** 完整轨迹模型 */
export interface TrajectoryModel {
  /** 本次请求实际使用的协议（分发解析器的依据） */
  protocol: ApiProtocolName;
  /** 内容可用性提示（原文缺失时非空，视图顶部展示） */
  warnings: string[];
  turns: TrajectoryTurn[];
  /** 扁平记录列表（选择、搜索、时间轴共用） */
  cells: TrajectoryCell[];
  timeline: TrajectoryTimeline;
  stats: TrajectoryStats;
}

/** 协议中文标签 */
export const PROTOCOL_LABELS: Record<string, string> = {
  anthropic: 'Anthropic',
  openai: 'OpenAI Chat',
  openai_response: 'OpenAI Responses',
};

/** 泳道中文标签 */
export const LANE_LABELS: Record<TrajectoryLane, string> = {
  input: '输入',
  model: '模型',
  tool: '工具',
};

/** 记录类型中文标签 */
export const KIND_LABELS: Record<TrajectoryCellKind, string> = {
  request: '请求',
  response: '响应',
  system: '系统',
  user: '用户',
  assistant: '助手',
  tool: '工具',
  raw: '原文',
};

/** 行摘要与预览的截断长度 */
const PREVIEW_LENGTH = 160;

/** 工具入参内联预览长度 */
const ARGS_INLINE_LENGTH = 70;

/** 原文未落库时，检查器「原始内容」页签的说明文案 */
const MISSING_CONTENT_HINT =
  '该日志未保存原文：log_contents 无对应记录（原始内容按月分区存放，' +
  '分区被清理或数据库重建后历史行会缺失，而 log_requests 长期保留）。';

// ─── 构建期中间结构 ───

/** 工具调用（与结果配对前的形态） */
interface PendingCall {
  callId: string;
  toolName: string;
  params: unknown;
  raw: unknown;
  result?: string;
  resultRaw?: unknown;
  isError: boolean;
}

/** 请求体解析出的原始事件 */
type RequestEvent =
  | { kind: 'system'; text: string; raw: unknown; label: string }
  | { kind: 'user'; text: string; raw: unknown; opensTurn: boolean }
  | {
      kind: 'assistant';
      text: string;
      thinking: string;
      raw: unknown;
      calls: PendingCall[];
    }
  | { kind: 'tool'; call: PendingCall }
  | { kind: 'tool_result'; callId: string; text: string; isError: boolean; raw: unknown };

/** 记录行草稿（尚未编号） */
type CellDraft = Omit<TrajectoryCell, 'index'>;

/** 分组草稿 */
interface GroupDraft {
  key: string;
  title: string;
  description?: string;
  cells: CellDraft[];
}

/** 轮次草稿 */
interface TurnDraft {
  index: number;
  title: string;
  description?: string;
  groups: GroupDraft[];
}

// ─── 主入口 ───

/**
 * 构建单条日志的运行轨迹
 *
 * @param data - 日志完整详情（请求体、响应体、头部、词元与耗时）
 * @returns 轮次账本 + 扁平行列表 + 时间轴投影
 */
export function buildTrajectory(data: LogDetailFull): TrajectoryModel {
  // 请求体可能是对象，也可能是被双重编码的 JSON 字符串（历史数据/上游写入差异）
  const body = decodeRequestBody(data.request_body);
  const protocol = resolveProtocol(data, body);
  const events = collectRequestEvents(body, protocol);

  // 1. 请求体历史装配为轮次 → 分组 → 记录行
  const drafts = assembleTurns(events);

  // 2. 追加本次请求的请求行、响应行与待回传结果的工具行
  appendCurrentRequest(drafts, data, protocol, body);

  // 3. 全局编号并物化为最终模型
  const cells: TrajectoryCell[] = [];
  const turns: TrajectoryTurn[] = drafts.map((turn) => ({
    index: turn.index,
    title: turn.title,
    description: turn.description,
    groups: turn.groups.map((group) => ({
      key: group.key,
      title: group.title,
      description: group.description,
      cells: group.cells.map((draft) => {
        const cell: TrajectoryCell = { ...draft, index: cells.length + 1 };
        cells.push(cell);
        return cell;
      }),
    })),
  }));

  return {
    protocol,
    warnings: buildWarnings(data, body),
    turns,
    cells,
    timeline: projectTimeline(turns, cells),
    stats: {
      turns: turns.length,
      steps: turns.reduce(
        (sum, turn) => sum + turn.groups.filter((group) => group.title !== '消息').length,
        0,
      ),
      cells: cells.length,
      messages: cells.filter(
        (cell) => cell.kind === 'user' || cell.kind === 'assistant' || cell.kind === 'system',
      ).length,
      toolCalls: cells.filter((cell) => cell.kind === 'tool').length,
    },
  };
}

/**
 * 日志是否还留有可还原轨迹的原文
 *
 * 请求体与响应体**两侧都没有**时，轨迹只剩两个空信封行 + 一条没有信息的游标，
 * 继续绘制会误导阅读（让人以为这就是全部记录）；此时详情页应改画说明卡。
 * 只有一侧缺失时仍渲染轨迹，由警告条与「无原文」标签标出缺口。
 */
export function hasTrajectoryContent(data: LogDetailFull): boolean {
  const hasRequest = Object.keys(decodeRequestBody(data.request_body)).length > 0;
  const hasResponse = typeof data.response_body === 'string' && data.response_body.length > 0;

  if (!hasRequest && !hasResponse) return false;
  // 后端明确标记原文未落库时，只有请求/响应体任一实际存在才继续渲染
  if (data.content_available === false) return hasRequest || hasResponse;
  return true;
}

/**
 * 请求体解码：兼容对象与被双重编码的 JSON 字符串
 *
 * 正常链路写入的是 JSONB 对象；历史数据或异常写入可能是 JSON 字符串
 * （`"{\"messages\":…}"`），此时直接当对象读会得到 0 消息 0 工具。
 */
function decodeRequestBody(value: unknown): Record<string, unknown> {
  if (value === null || value === undefined) return {};

  if (typeof value === 'string') {
    try {
      const parsed: unknown = JSON.parse(value);
      return isRecord(parsed) ? parsed : {};
    } catch {
      return {};
    }
  }

  return isRecord(value) ? value : {};
}

/** 是否普通对象（非数组、非 null） */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/**
 * 协议判定（三族响应形态各自解析，这里只决定用哪一族）
 *
 * 1. **请求体形状优先**：`input` 数组 / `instructions` 字符串是 Responses 的特征，
 *    与接入点家族无关（OpenAI 家族一个接入点同时服务 Chat 与 Responses 两条线）
 * 2. `api_protocol` 为 `openai` / `openai_response` 时直接采用
 * 3. 回退接入点家族 `api_type`：只有 OpenAI 家族会走 OpenAI 解析器
 *
 * 注意 `api_protocol` 列的默认值是 `'anthropic'`，旧数据或是回溯迁移漏掉的行
 * 不宜当作可靠依据，所以它落在第 3 步之后。
 */
function resolveProtocol(data: LogDetailFull, body: Record<string, unknown>): ApiProtocolName {
  if (Array.isArray(body.input) || typeof body.instructions === 'string') {
    return 'openai_response';
  }
  if (data.api_protocol === 'openai' || data.api_protocol === 'openai_response') {
    return data.api_protocol;
  }
  return data.api_type === 'openai' ? 'openai' : 'anthropic';
}

/** 内容可用性提示：区分「原文未落库」与「原文存在但无法解析」 */
function buildWarnings(data: LogDetailFull, body: Record<string, unknown>): string[] {
  const hasRequest = Object.keys(body).length > 0;
  const hasResponse = typeof data.response_body === 'string' && data.response_body.length > 0;
  const contentMissing = data.content_available === false;
  const warnings: string[] = [];

  // 两侧都缺：详情页改画说明卡（见 `hasTrajectoryContent`），此处只留一条概述性提示
  if (!hasRequest && !hasResponse) {
    return [
      contentMissing
        ? '该日志的请求 / 响应原文未保存（log_contents 无对应记录），内容无法还原'
        : '该日志没有请求体与响应体原文，内容无法还原',
    ];
  }

  // 只缺一侧：按侧分别说明，轨迹仍渲染但标出缺口
  if (!hasRequest) {
    warnings.push(
      contentMissing
        ? '该日志的请求体原文未保存（log_contents 无对应记录），请求内容无法还原'
        : '该日志没有请求体原文，请求内容无法还原',
    );
  }
  if (!hasResponse) {
    warnings.push(
      contentMissing
        ? '该日志的响应体原文未保存（log_contents 无对应记录），响应内容无法还原'
        : '该日志没有响应体原文，响应内容无法还原',
    );
  }
  return warnings;
}

/** 按协议收集请求体内的原始事件（不含本次响应） */
function collectRequestEvents(
  body: Record<string, unknown>,
  protocol: ApiProtocolName,
): RequestEvent[] {
  if (protocol === 'openai') return collectOpenAIChatEvents(body.messages);
  if (protocol === 'openai_response') return collectOpenAIResponsesEvents(body);
  return collectAnthropicEvents(body);
}

/** Anthropic：`system` + `messages`（含 thinking / tool_use / tool_result） */
function collectAnthropicEvents(body: Record<string, unknown>): RequestEvent[] {
  const events: RequestEvent[] = [];

  const system = contentToText(body.system).trim();
  if (system) {
    events.push({ kind: 'system', text: system, raw: body.system, label: '系统提示' });
  }

  if (!Array.isArray(body.messages)) return events;

  for (const rawMessage of body.messages) {
    const message = rawMessage as Record<string, unknown>;
    const role = String(message.role ?? 'unknown');
    const content = message.content;

    // 1. 纯文本消息
    if (typeof content === 'string') {
      const text = content.trim();
      if (!text) continue;
      if (role === 'assistant') {
        events.push({
          kind: 'assistant',
          text,
          thinking: '',
          raw: message,
          calls: [],
        });
      } else {
        events.push({ kind: 'user', text, raw: message, opensTurn: true });
      }
      continue;
    }

    if (!Array.isArray(content)) continue;

    // 2. 内容块消息：助手块合并为一条助手事件，用户块各自成条
    const assistantText: string[] = [];
    const assistantThinking: string[] = [];
    const calls: PendingCall[] = [];
    const userBlocks: Array<{ text: string; raw: unknown }> = [];

    for (const rawBlock of content) {
      const block = rawBlock as Record<string, unknown>;
      const type = String(block.type ?? '');

      if (type === 'text') {
        const text = typeof block.text === 'string' ? block.text.trim() : '';
        if (!text) continue;
        if (role === 'assistant') assistantText.push(text);
        else userBlocks.push({ text, raw: block });
      } else if (type === 'thinking' || type === 'redacted_thinking') {
        const text = contentToText(block.thinking ?? block.text).trim();
        if (text) assistantThinking.push(text);
      } else if (type === 'tool_use') {
        calls.push({
          callId: typeof block.id === 'string' ? block.id : '',
          toolName: typeof block.name === 'string' ? block.name : '未知工具',
          params: block.input ?? {},
          raw: block,
          isError: false,
        });
      } else if (type === 'tool_result') {
        events.push({
          kind: 'tool_result',
          callId: typeof block.tool_use_id === 'string' ? block.tool_use_id : '',
          text: contentToText(block.content).trim(),
          isError: block.is_error === true,
          raw: block,
        });
      } else if (type === 'image') {
        userBlocks.push({ text: `[图片] ${describeImage(block.source)}`.trim(), raw: block });
      } else if (type) {
        userBlocks.push({ text: `[${type}]`, raw: block });
      }
    }

    if (role === 'assistant') {
      if (assistantText.length > 0 || assistantThinking.length > 0 || calls.length > 0) {
        events.push({
          kind: 'assistant',
          text: assistantText.join('\n\n'),
          thinking: assistantThinking.join('\n\n'),
          raw: message,
          calls,
        });
      }
      continue;
    }

    // 用户消息：只有首个块开启新轮次；仅含 tool_result 时不开新轮次（属于上一轮续接）
    userBlocks.forEach((block, position) => {
      events.push({
        kind: 'user',
        text: block.text,
        raw: block.raw,
        opensTurn: position === 0,
      });
    });
  }

  return pairToolResults(events);
}

/** OpenAI Chat Completions：`messages`（含 tool_calls 与 role = tool） */
function collectOpenAIChatEvents(messages: unknown): RequestEvent[] {
  if (!Array.isArray(messages)) return [];

  const events: RequestEvent[] = [];

  for (const rawMessage of messages) {
    const message = rawMessage as Record<string, unknown>;
    const role = String(message.role ?? 'unknown');
    const text = contentToText(message.content).trim();

    if (role === 'system' || role === 'developer') {
      if (text) {
        events.push({
          kind: 'system',
          text,
          raw: message,
          label: role === 'developer' ? '开发者指令' : '系统提示',
        });
      }
      continue;
    }

    if (role === 'tool') {
      events.push({
        kind: 'tool_result',
        callId: typeof message.tool_call_id === 'string' ? message.tool_call_id : '',
        text,
        isError: false,
        raw: message,
      });
      continue;
    }

    if (role === 'assistant') {
      const calls: PendingCall[] = [];
      const rawCalls = message.tool_calls;
      if (Array.isArray(rawCalls)) {
        for (const rawCall of rawCalls) {
          const call = rawCall as Record<string, unknown>;
          const fn = (call.function ?? {}) as Record<string, unknown>;
          calls.push({
            callId: typeof call.id === 'string' ? call.id : '',
            toolName: typeof fn.name === 'string' ? fn.name : '未知工具',
            params: parseJsonLike(fn.arguments),
            raw: call,
            isError: false,
          });
        }
      }
      if (text || calls.length > 0) {
        events.push({ kind: 'assistant', text, thinking: '', raw: message, calls });
      }
      continue;
    }

    if (text) {
      events.push({ kind: 'user', text, raw: message, opensTurn: true });
    }
  }

  return pairToolResults(events);
}

/** OpenAI Responses：`instructions` + `input` 项 */
function collectOpenAIResponsesEvents(body: Record<string, unknown>): RequestEvent[] {
  const events: RequestEvent[] = [];

  const instructions = typeof body.instructions === 'string' ? body.instructions.trim() : '';
  if (instructions) {
    events.push({ kind: 'system', text: instructions, raw: instructions, label: '系统指示' });
  }

  if (!Array.isArray(body.input)) return events;

  for (const rawItem of body.input) {
    const item = rawItem as Record<string, unknown>;
    const type = String(item.type ?? '');

    // 1. 角色消息（message / 带 role 的项）
    if (item.role || type === 'message') {
      const role = String(item.role ?? 'user');
      const text = contentToText(item.content).trim();
      if (!text) continue;
      if (role === 'assistant') {
        events.push({ kind: 'assistant', text, thinking: '', raw: item, calls: [] });
      } else if (role === 'user') {
        events.push({ kind: 'user', text, raw: item, opensTurn: true });
      } else {
        events.push({ kind: 'system', text, raw: item, label: '系统指示' });
      }
      continue;
    }

    if (type === 'function_call') {
      events.push({
        kind: 'tool',
        call: {
          callId: typeof item.call_id === 'string' ? item.call_id : '',
          toolName: typeof item.name === 'string' ? item.name : '未知工具',
          params: parseJsonLike(item.arguments),
          raw: item,
          isError: false,
        },
      });
      continue;
    }

    if (type === 'function_call_output') {
      events.push({
        kind: 'tool_result',
        callId: typeof item.call_id === 'string' ? item.call_id : '',
        text: typeof item.output === 'string' ? item.output : contentToText(item.output),
        isError: String(item.status ?? '') === 'error',
        raw: item,
      });
      continue;
    }

    if (type === 'reasoning') {
      const thinking = contentToText(item.summary) || contentToText(item.content);
      if (thinking) {
        events.push({ kind: 'assistant', text: '', thinking, raw: item, calls: [] });
      }
    }
  }

  return pairToolResults(events);
}

/** 把 tool_result 事件按 callId 配对进工具调用；未配对的保留为独立事件 */
function pairToolResults(events: RequestEvent[]): RequestEvent[] {
  const callsById = new Map<string, PendingCall>();
  for (const event of events) {
    if (event.kind === 'assistant') {
      for (const call of event.calls) {
        if (call.callId) callsById.set(call.callId, call);
      }
    } else if (event.kind === 'tool' && event.call.callId) {
      callsById.set(event.call.callId, event.call);
    }
  }

  const paired: RequestEvent[] = [];
  for (const event of events) {
    if (event.kind !== 'tool_result') {
      paired.push(event);
      continue;
    }
    const target = event.callId ? callsById.get(event.callId) : undefined;
    if (target) {
      target.result = event.text;
      target.resultRaw = event.raw;
      target.isError = event.isError;
    } else {
      // 找不到调用方的结果（历史被截断）保留为独立工具行，不丢信息
      paired.push(event);
    }
  }
  return paired;
}

// ─── 轮次装配 ───

/** 把请求体事件装配为「轮次 → 分组 → 记录行」草稿 */
function assembleTurns(events: RequestEvent[]): TurnDraft[] {
  const turns: TurnDraft[] = [];
  let messageGroup: GroupDraft | null = null;
  let stepNo = 0;
  // 先于任何用户消息出现的系统提示：挂到首个轮次的消息分组，不单独成轮
  const pendingSystem: CellDraft[] = [];

  /** 开一个新轮次并返回其消息分组（顺带安置待挂的系统提示） */
  const openTurn = (description?: string): GroupDraft => {
    const turn: TurnDraft = {
      index: turns.length + 1,
      title: `第 ${turns.length + 1} 轮`,
      description,
      groups: [],
    };
    turns.push(turn);
    messageGroup = { key: `${turn.index}-message`, title: '消息', cells: [...pendingSystem] };
    pendingSystem.length = 0;
    turn.groups.push(messageGroup);
    stepNo = 0;
    return messageGroup;
  };

  /** 取当前轮次的消息分组（不存在时补建在最前） */
  const currentMessageGroup = (turn: TurnDraft): GroupDraft => {
    if (!messageGroup) {
      messageGroup = { key: `${turn.index}-message`, title: '消息', cells: [] };
      turn.groups.unshift(messageGroup);
    }
    return messageGroup;
  };

  /** 取当前轮次（不存在时开一个新轮次） */
  const currentTurn = (): TurnDraft => {
    if (turns.length === 0) openTurn();
    return turns[turns.length - 1];
  };

  /** 追加一个「第 k 步」分组 */
  const pushStep = (turn: TurnDraft, description?: string): GroupDraft => {
    stepNo += 1;
    const group: GroupDraft = {
      key: `${turn.index}-step-${stepNo}`,
      title: `第 ${stepNo} 步`,
      description,
      cells: [],
    };
    turn.groups.push(group);
    return group;
  };

  for (const event of events) {
    if (event.kind === 'user' && event.opensTurn) {
      openTurn(firstLine(event.text, PREVIEW_LENGTH)).cells.push(userCell(event.text, event.raw));
      continue;
    }

    // 系统提示先于首个用户消息时暂存，等轮次建立后并入其消息分组
    if (event.kind === 'system' && turns.length === 0) {
      pendingSystem.push(systemCell(event.label, event.text, event.raw));
      continue;
    }

    const turn = currentTurn();

    if (event.kind === 'system') {
      currentMessageGroup(turn).cells.push(systemCell(event.label, event.text, event.raw));
      continue;
    }

    if (event.kind === 'user') {
      currentMessageGroup(turn).cells.push(userCell(event.text, event.raw));
      continue;
    }

    if (event.kind === 'assistant') {
      const group = pushStep(turn);
      group.cells.push(assistantCell(event));
      for (const call of event.calls) {
        group.cells.push(toolCell(call));
      }
      continue;
    }

    if (event.kind === 'tool') {
      const group = pushStep(turn);
      group.cells.push(toolCell(event.call));
      continue;
    }

    const group = pushStep(turn);
    group.cells.push(orphanToolResultCell(event));
  }

  // 只有系统提示、没有任何用户消息时，也建立一轮承载它们（本次请求随后并入该轮）
  if (pendingSystem.length > 0) openTurn();

  return turns;
}

/** 追加本次请求的请求行、响应行与待回传结果的工具行 */
function appendCurrentRequest(
  turns: TurnDraft[],
  data: LogDetailFull,
  protocol: ApiProtocolName,
  body: Record<string, unknown>,
): void {
  if (turns.length === 0) {
    turns.push({ index: 1, title: '第 1 轮', groups: [] });
  }
  const turn = turns[turns.length - 1];
  const stepCount = turn.groups.filter((group) => group.title !== '消息').length;
  const group: GroupDraft = {
    key: `${turn.index}-step-${stepCount + 1}-request`,
    title: `第 ${stepCount + 1} 步`,
    description: '本次请求',
    cells: [],
  };
  turn.groups.push(group);

  const hasRequest = Object.keys(body).length > 0;
  const hasResponse = typeof data.response_body === 'string' && data.response_body.length > 0;

  // 1. 请求行（检查器内含协议、选项、请求头、原始请求体）
  group.cells.push(requestCell(data, protocol, body, hasRequest));

  // 2. 响应行：解析出的块折叠进一条助手记录；原文缺失时如实标注
  group.cells.push(buildResponseCell(data, protocol, hasResponse));

  // 3. 本次响应发起的工具调用（结果尚未回传）
  if (hasResponse) group.cells.push(...buildPendingToolCells(data, protocol));
}

// ─── 记录行构造 ───

/** 系统提示行 */
function systemCell(label: string, text: string, raw: unknown): CellDraft {
  return {
    kind: 'system',
    kindLabel: KIND_LABELS.system,
    text: label,
    preview: firstLine(text, PREVIEW_LENGTH),
    timeMs: null,
    status: 'completed',
    isError: false,
    lane: 'input',
    tabs: [
      tabOverview([
        { label: '层级', value: '系统提示' },
        { label: '字符数', value: formatNumberText(text.length) },
      ]),
      tabText('content', '内容', text),
      tabJson('raw', '原始内容', raw),
    ],
  };
}

/** 用户消息行 */
function userCell(text: string, raw: unknown): CellDraft {
  return {
    kind: 'user',
    kindLabel: KIND_LABELS.user,
    text: firstLine(text, PREVIEW_LENGTH),
    preview: remainder(text),
    timeMs: null,
    status: 'completed',
    isError: false,
    lane: 'input',
    tabs: [
      tabOverview([
        { label: '层级', value: '用户消息' },
        { label: '字符数', value: formatNumberText(text.length) },
      ]),
      tabText('content', '内容', text),
      tabJson('raw', '原始内容', raw),
    ],
  };
}

/** 历史助手消息行（thinking 与正文折叠为同一记录的页签） */
function assistantCell(event: {
  text: string;
  thinking: string;
  raw: unknown;
  calls: PendingCall[];
}): CellDraft {
  const hasProse = event.text.trim().length > 0;

  return {
    kind: 'assistant',
    kindLabel: KIND_LABELS.assistant,
    text: hasProse ? firstLine(event.text, PREVIEW_LENGTH) : '（仅工具调用）',
    preview: hasProse ? remainder(event.text) : undefined,
    timeMs: null,
    status: 'completed',
    isError: false,
    lane: 'model',
    tabs: [
      tabOverview([
        { label: '层级', value: '助手消息' },
        { label: '状态', value: '已完成' },
        { label: '工具调用', value: `${event.calls.length} 次` },
        ...(event.thinking
          ? [{ label: '思考字符数', value: formatNumberText(event.thinking.length) }]
          : []),
      ]),
      ...(hasProse ? [tabText('output', '原始输出', event.text)] : []),
      ...(event.thinking ? [tabText('thinking', '思考', event.thinking)] : []),
      tabJson('raw', '原始内容', event.raw),
    ],
  };
}

/** 工具调用行（与结果配对在同一行） */
function toolCell(call: PendingCall): CellDraft {
  const argsInline = compactJson(call.params, ARGS_INLINE_LENGTH);
  const resultText = call.result ?? '';
  const status: TrajectoryStatus = call.result === undefined ? 'pending' : 'completed';

  return {
    kind: 'tool',
    kindLabel: KIND_LABELS.tool,
    text: `${call.toolName} ${argsInline}`.trim(),
    preview: resultText ? firstLine(resultText, PREVIEW_LENGTH) : '未捕获结果',
    toolName: call.toolName,
    callId: call.callId,
    timeMs: null,
    status: call.isError ? 'failed' : status,
    isError: call.isError,
    lane: 'tool',
    tabs: [
      tabOverview([
        { label: '工具', value: call.toolName || '未知工具' },
        { label: '调用 ID', value: call.callId || '未记录', mono: true },
        {
          label: '状态',
          value: call.isError ? '失败' : status === 'pending' ? '等待中' : '已完成',
        },
      ]),
      tabJson('payload', '参数', call.params),
      tabText('result', '结果', resultText || '未捕获结果'),
      tabJson('raw', '原始内容', { call: call.raw, result: call.resultRaw ?? null }),
    ],
  };
}

/** 孤立工具结果行（找不到调用方，如历史被截断） */
function orphanToolResultCell(event: {
  callId: string;
  text: string;
  isError: boolean;
  raw: unknown;
}): CellDraft {
  return {
    kind: 'tool',
    kindLabel: KIND_LABELS.tool,
    text: '工具结果',
    preview: firstLine(event.text, PREVIEW_LENGTH),
    callId: event.callId,
    timeMs: null,
    status: event.isError ? 'failed' : 'completed',
    isError: event.isError,
    lane: 'tool',
    tabs: [
      tabOverview([
        { label: '层级', value: '工具结果' },
        { label: '调用 ID', value: event.callId || '未记录', mono: true },
        { label: '状态', value: event.isError ? '失败' : '已完成' },
      ]),
      tabText('result', '结果', event.text),
      tabJson('raw', '原始内容', event.raw),
    ],
  };
}

/** 本次请求的请求行 */
function requestCell(
  data: LogDetailFull,
  protocol: ApiProtocolName,
  body: Record<string, unknown>,
  hasBody: boolean,
): CellDraft {
  const tools = Array.isArray(body.tools) ? body.tools.length : 0;
  const messages = Array.isArray(body.messages)
    ? body.messages.length
    : Array.isArray(body.input)
      ? body.input.length
      : 0;

  const overview: TrajectoryMetaItem[] = [
    { label: '层级', value: '本次请求' },
    { label: '协议', value: PROTOCOL_LABELS[protocol] ?? protocol },
    { label: '状态', value: '已完成' },
    { label: '模型', value: data.model_mapped || data.model_original || '-' },
    { label: '内容可用性', value: hasBody ? '请求体原文已保存' : '请求体原文未保存' },
  ];
  if (hasBody) {
    overview.push(
      { label: '消息条数', value: `${messages} 条` },
      { label: '工具定义', value: `${tools} 个` },
    );
  }
  if (data.model_original && data.model_mapped) {
    overview.push({ label: '模型映射', value: `${data.model_original} → ${data.model_mapped}` });
  }
  if (data.access_point_name || data.access_point_id) {
    overview.push({
      label: '接入点',
      value: data.access_point_name ?? data.access_point_id ?? '-',
    });
  }
  if (data.session_id) overview.push({ label: '会话 ID', value: data.session_id, mono: true });

  const opts = requestOptions(body);
  const headers = headerText(data.request_headers);

  return {
    kind: 'request',
    kindLabel: KIND_LABELS.request,
    text: `请求 #1`,
    preview: hasBody
      ? `${PROTOCOL_LABELS[protocol] ?? protocol} · ${messages} 条消息 · ${tools} 个工具`
      : '未保存请求体原文（无法还原请求内容）',
    model: data.model_mapped || data.model_original || undefined,
    time: formatClock(data.timestamp),
    timeMs: data.duration_ms ?? null,
    status: 'completed',
    isError: false,
    contentMissing: !hasBody,
    lane: 'model',
    tabs: [
      tabOverview(overview),
      ...(Object.keys(opts).length > 0 ? [tabJson('options', '选项', opts)] : []),
      ...(headers ? [tabText('headers', '请求头', headers)] : []),
      ...(hasBody
        ? [tabJson('raw', '原始内容', data.request_body ?? null)]
        : [tabText('raw', '原始内容', MISSING_CONTENT_HINT)]),
    ],
  };
}

/** 本次请求的响应行 */
function buildResponseCell(
  data: LogDetailFull,
  protocol: ApiProtocolName,
  hasBody: boolean,
): CellDraft {
  const body = data.response_body;
  if (!hasBody || !body) return responseFallbackCell(data, '', true);

  const format = detectResponseFormat(data.response_headers as Record<string, unknown> | null);
  const blocks = parseResponseBlocks(body, format, protocol);
  if (blocks.length === 0) return responseFallbackCell(data, body, false);

  const text = joinBlocks(blocks, 'text');
  const thinking = joinBlocks(blocks, 'thinking');

  return progressAssistantCell(data, text, thinking);
}

/** 响应中已解析块的文本拼接 */
function joinBlocks(blocks: ContentBlockInfo[], kind: 'text' | 'thinking'): string {
  return blocks
    .filter((block) => block.block_type === kind)
    .map((block) => (kind === 'text' ? (block.text ?? '') : (block.thinking ?? '')))
    .join('\n\n')
    .trim();
}

/**
 * 本次请求的响应行：把响应解析出的 thinking / 正文折叠进一条助手记录
 *
 * 与 DSH 的 message 记录一致——思考与正文是同一助手记录的两个检查器页签，
 * 工具调用则各自成行（见 `buildPendingToolCells`）。
 */
function progressAssistantCell(data: LogDetailFull, text: string, thinking: string): CellDraft {
  const failed = (data.status_code ?? 0) >= 400 || Boolean(data.error_message);
  const usage = tokenUsage(data);
  const hasProse = text.length > 0;
  const headers = headerText(data.response_headers);

  const overview: TrajectoryMetaItem[] = [
    { label: '层级', value: '助手消息' },
    { label: '状态码', value: String(data.status_code ?? '-') },
    { label: '状态', value: failed ? '失败' : '已完成' },
    { label: '模型', value: data.model_mapped || data.model_original || '-' },
    { label: '内容可用性', value: '响应体原文已保存' },
    ...usageItems(data),
  ];
  if (data.error_message) overview.push({ label: '错误', value: data.error_message });

  return {
    kind: 'assistant',
    kindLabel: KIND_LABELS.assistant,
    text: hasProse ? firstLine(text, PREVIEW_LENGTH) : '（仅工具调用）',
    preview: hasProse ? remainder(text) : undefined,
    model: data.model_mapped || data.model_original || undefined,
    time: formatClock(data.timestamp),
    // 助手行不挂耗时：日志只记整条请求耗时，挂在消息上属于误导（请求/响应信封行保留真实值）
    timeMs: null,
    input: usage.input,
    cacheRead: usage.cacheRead,
    cacheWrite: usage.cacheWrite,
    output: usage.output,
    think: usage.think,
    status: failed ? 'failed' : 'completed',
    isError: failed,
    lane: 'model',
    tabs: [
      tabOverview(overview),
      ...(hasProse ? [tabText('output', '原始输出', text)] : []),
      ...(thinking ? [tabText('thinking', '思考', thinking)] : []),
      ...(headers ? [tabText('headers', '响应头', headers)] : []),
      tabJson('raw', '原始内容', parseRawJson(data.response_body)),
    ],
  };
}

/**
 * 响应原文缺失或无法解析为结构化块时的响应行
 *
 * `missing = true` 表示原文未落库（内容是缺失的，不是解析失败）；
 * `missing = false` 表示原文存在但没解析出结构化内容（错误响应、非预期形态）。
 */
function responseFallbackCell(data: LogDetailFull, rawBody: string, missing: boolean): CellDraft {
  const failed = (data.status_code ?? 0) >= 400 || Boolean(data.error_message);
  const usage = tokenUsage(data);
  const headers = headerText(data.response_headers);

  const overview: TrajectoryMetaItem[] = [
    { label: '层级', value: missing ? '本次响应' : '本次响应（未解析出结构化内容）' },
    { label: '状态码', value: String(data.status_code ?? '-') },
    { label: '状态', value: failed ? '失败' : '已完成' },
    { label: '耗时', value: `${formatNumberText(data.duration_ms ?? 0)} 毫秒` },
    { label: '内容可用性', value: missing ? '响应体原文未保存' : '响应体原文已保存' },
    ...usageItems(data),
  ];
  if (data.error_message) overview.push({ label: '错误', value: data.error_message });

  return {
    kind: 'response',
    kindLabel: KIND_LABELS.response,
    text: '响应',
    preview: missing
      ? '未保存响应体原文（无法还原响应内容）'
      : firstLine(rawBody, PREVIEW_LENGTH) || '无响应内容',
    time: formatClock(data.timestamp),
    timeMs: data.duration_ms ?? null,
    input: usage.input,
    cacheRead: usage.cacheRead,
    cacheWrite: usage.cacheWrite,
    output: usage.output,
    think: usage.think,
    status: failed ? 'failed' : 'completed',
    isError: failed,
    contentMissing: missing,
    lane: 'model',
    tabs: [
      tabOverview(overview),
      ...(rawBody ? [tabText('result', '结果', rawBody)] : []),
      ...(headers ? [tabText('headers', '响应头', headers)] : []),
      ...(missing
        ? [tabText('raw', '原始内容', MISSING_CONTENT_HINT)]
        : [tabJson('raw', '原始内容', parseRawJson(rawBody))]),
    ],
  };
}

/** 本次响应发起的工具调用（客户端尚未回传结果） */
function buildPendingToolCells(data: LogDetailFull, protocol: ApiProtocolName): CellDraft[] {
  const body = data.response_body;
  if (!body) return [];

  const format = detectResponseFormat(data.response_headers as Record<string, unknown> | null);
  const blocks = parseResponseBlocks(body, format, protocol);

  return blocks
    .filter((block) => block.block_type === 'tool_use')
    .map((block) => {
      const argsInline = compactJson(block.input ?? {}, ARGS_INLINE_LENGTH);
      return {
        kind: 'tool' as const,
        kindLabel: KIND_LABELS.tool,
        text: `${block.tool_name ?? '未知工具'} ${argsInline}`.trim(),
        preview: '等待客户端回传结果',
        toolName: block.tool_name,
        callId: block.tool_use_id,
        timeMs: null,
        status: 'pending' as const,
        isError: false,
        lane: 'tool' as const,
        tabs: [
          tabOverview([
            { label: '工具', value: block.tool_name ?? '未知工具' },
            { label: '调用 ID', value: block.tool_use_id ?? '未记录', mono: true },
            { label: '状态', value: '等待中' },
          ]),
          tabJson('payload', '参数', block.input ?? {}),
          tabText('result', '结果', '未捕获结果'),
          tabJson('raw', '原始内容', block),
        ],
      };
    });
}

// ─── 时间轴投影 ───

/**
 * 三泳道时间轴投影
 *
 * 以记录出现顺序为轴（序列投影）：每条记录落在自己的槽位上，宽度按内容体量的
 * 相对大小绘制（日志未记录逐条耗时，因此不做时间维度的排布）。
 * 轮次边界给出每个轮次首条记录的位置，供时间轴画分隔线。
 */
function projectTimeline(turns: TrajectoryTurn[], cells: TrajectoryCell[]): TrajectoryTimeline {
  const turnOfCell = new Map<number, number>();
  for (const turn of turns) {
    for (const group of turn.groups) {
      for (const cell of group.cells) {
        turnOfCell.set(cell.index, turn.index);
      }
    }
  }

  const spans: TrajectoryTimelineSpan[] = [];
  const boundaries: TrajectoryTimelineBoundary[] = [];
  const seenTurns = new Set<number>();

  // 只有带泳道的记录进时间轴；槽位按可见顺序连续编号 → 等宽且互不重叠
  for (const cell of cells) {
    if (!cell.lane) continue;
    const slot = spans.length;
    const turnIndex = turnOfCell.get(cell.index) ?? null;
    spans.push({
      cellIndex: cell.index,
      lane: cell.lane,
      slot,
      kind: cell.kind,
      label: `${cell.kindLabel} #${cell.index}`,
      turnIndex,
    });
    if (turnIndex !== null && !seenTurns.has(turnIndex)) {
      seenTurns.add(turnIndex);
      boundaries.push({ turnIndex, slot });
    }
  }

  return { spans, boundaries, total: spans.length };
}

/**
 * 槽位 → 相对位置（0..1）
 *
 * 每条记录一个等宽槽位：`[slot/total, (slot+1)/total)`。
 * 槽位互斥，故不同泳道的片段也不会在横向上重叠。
 */
export function trajectorySpanBox(
  span: TrajectoryTimelineSpan,
  timeline: TrajectoryTimeline,
): { left: number; width: number } {
  const total = Math.max(1, timeline.total);
  return { left: span.slot / total, width: 1 / total };
}

// ─── 页签与格式化辅助 ───

/** 概述页签 */
function tabOverview(items: TrajectoryMetaItem[]): TrajectoryTab {
  return { key: 'overview', label: '概述', kind: 'meta', items };
}

/** JSON 页签 */
function tabJson(key: TrajectoryTabKey, label: string, value: unknown): TrajectoryTab {
  return { key, label, kind: 'json', value: safeStringify(value) };
}

/** 文本页签 */
function tabText(key: TrajectoryTabKey, label: string, value: string): TrajectoryTab {
  return { key, label, kind: 'text', value };
}

/** 词元（与后端 6 列对齐） */
function tokenUsage(data: LogDetailFull): {
  input: number;
  cacheRead: number;
  cacheWrite: number;
  output: number;
  think: number;
  total: number;
} {
  return {
    input: data.token_input_tokens ?? 0,
    cacheRead: data.token_cache_read_input_tokens ?? 0,
    cacheWrite: data.token_cache_creation_input_tokens ?? 0,
    output: data.token_output_tokens ?? 0,
    think: data.token_thinking_tokens ?? 0,
    total: data.token_total_tokens ?? 0,
  };
}

/** 词元的概述条目 */
function usageItems(data: LogDetailFull): TrajectoryMetaItem[] {
  const usage = tokenUsage(data);
  const items: TrajectoryMetaItem[] = [{ label: '总计词元', value: formatNumberText(usage.total) }];
  if (usage.input > 0) items.push({ label: '输入', value: formatNumberText(usage.input) });
  if (usage.cacheWrite > 0) {
    items.push({ label: '缓存写入', value: formatNumberText(usage.cacheWrite) });
  }
  if (usage.cacheRead > 0) {
    items.push({ label: '缓存读取', value: formatNumberText(usage.cacheRead) });
  }
  if (usage.output > 0) items.push({ label: '输出', value: formatNumberText(usage.output) });
  if (usage.think > 0) items.push({ label: '思考', value: formatNumberText(usage.think) });
  return items;
}

/** 请求选项（生成参数，剔除消息与工具定义） */
function requestOptions(body: Record<string, unknown>): Record<string, unknown> {
  const skip = new Set(['messages', 'input', 'tools', 'system', 'instructions']);
  const options: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(body)) {
    if (skip.has(key) || value === null || value === undefined) continue;
    options[key] = value;
  }
  return options;
}

/** 头部键值对转可读文本 */
function headerText(headers: Record<string, unknown> | null | undefined): string {
  if (!headers) return '';
  const entries = Object.entries(headers);
  if (entries.length === 0) return '';
  return entries.map(([key, value]) => `${key}: ${String(value)}`).join('\n');
}

/** 响应原文解析为 JSON（失败时返回原文） */
function parseRawJson(body: string | null | undefined): unknown {
  if (!body) return null;
  try {
    return JSON.parse(body);
  } catch {
    return body;
  }
}

/** 单行摘要：压缩空白并截断 */
function firstLine(text: string, max: number): string {
  const single = text.replace(/\s+/g, ' ').trim();
  return single.length > max ? `${single.slice(0, max)}…` : single;
}

/** 摘要之后的余量（用于行的次级预览） */
function remainder(text: string): string | undefined {
  const single = text.replace(/\s+/g, ' ').trim();
  if (single.length <= PREVIEW_LENGTH) return undefined;
  return `${single.slice(PREVIEW_LENGTH, PREVIEW_LENGTH * 2)}…`;
}

/** 紧凑 JSON（无空白，超长截断） */
function compactJson(value: unknown, max: number): string {
  if (value === undefined || value === null) return '';
  let json: string;
  try {
    json = JSON.stringify(value);
  } catch {
    return '';
  }
  if (!json) return '';
  return json.length > max ? `${json.slice(0, max)}…` : json;
}

/** 安全序列化（失败时返回原文） */
function safeStringify(value: unknown): string {
  if (value === undefined || value === null) return '';
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

/** 时钟格式 HH:mm:ss（本地时间） */
function formatClock(timestamp: string | null | undefined): string {
  if (!timestamp) return '';
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) return '';
  return date.toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  });
}

/** 数字千分位 */
function formatNumberText(value: number): string {
  return value.toLocaleString('zh-CN');
}

/** 收敛到 0..1 */

/**
 * 把协议内容块（字符串 / 数组 / 对象）提炼为纯文本
 *
 * 覆盖 Anthropic（text / thinking / tool_result / image）与
 * OpenAI（input_text / output_text / image_url / summary_text）两族的文本形态。
 */
function contentToText(content: unknown): string {
  if (typeof content === 'string') return content;
  if (content === null || content === undefined) return '';

  if (Array.isArray(content)) {
    return content
      .map((part) => contentToText(part))
      .filter(Boolean)
      .join('\n');
  }

  if (typeof content === 'object') {
    const block = content as Record<string, unknown>;
    const type = String(block.type ?? '');

    if (typeof block.text === 'string') return block.text;
    if (typeof block.thinking === 'string') return block.thinking;
    if (typeof block.content === 'string' || Array.isArray(block.content)) {
      return contentToText(block.content);
    }
    if (type === 'image' || type === 'image_url') return '[图片]';
    if (typeof block.output === 'string') return block.output;
  }

  return '';
}

/** 图片块描述（media_type 等可读信息） */
function describeImage(source: unknown): string {
  if (!source || typeof source !== 'object') return '';
  const record = source as Record<string, unknown>;
  const mediaType = typeof record.media_type === 'string' ? record.media_type : '';
  return mediaType ? `(${mediaType})` : '';
}

/** 把 JSON 字符串解析为对象，解析失败时保留原文 */
function parseJsonLike(value: unknown): unknown {
  if (typeof value !== 'string') return value ?? {};
  try {
    return JSON.parse(value);
  } catch {
    return { raw_arguments: value };
  }
}
