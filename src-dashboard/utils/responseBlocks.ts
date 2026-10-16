/**
 * 响应内容解析分发模块
 *
 * 按请求级协议 `api_protocol`（缺失时回退接入点协议 `api_type`）把响应体
 * 分发到对应协议的解析器，统一输出 `ContentBlockInfo[]`，供对话轨迹渲染使用。
 * - `anthropic`（默认）：`parseStructuredBlocks`，SSE / JSON 自适应
 * - `openai`：Chat Completions 解析器，解析结果为空时用 Responses 解析器兜底
 * - `openai_response`：Responses 解析器
 */

import type { ContentBlockInfo, ResponseFormat } from './parseLogs.ts';
import { parseStructuredBlocks } from './parseLogs.ts';
import {
  parseOpenAIChatResponse,
  parseOpenAIChatSSE,
  parseOpenAIResponsesResponse,
  parseOpenAIResponsesSSE,
} from './parseOpenAI.ts';

/** 请求级协议名（与后端 `log_requests.api_protocol` 取值一致） */
export type ApiProtocolName = 'anthropic' | 'openai' | 'openai_response';

/**
 * 解析响应体为结构化 content blocks
 *
 * 三族响应形态各自独立解析，**不做形状互猜**：
 * - `anthropic`：`parseStructuredBlocks`（SSE `content_block_*` / JSON `content[]`）
 * - `openai`：Chat Completions（`choices[].message` / SSE `choices[].delta`）
 * - `openai_response`：Responses（`output[].content[]` / SSE `response.completed`）
 *
 * @param responseBody - 响应体原文（SSE 文本或 JSON 文本）
 * @param format - 响应体格式（SSE / JSON），由 `detectResponseFormat` 依据 Content-Type 判定
 * @param protocol - 已解析的请求级协议（由 `utils/trajectory.ts::resolveProtocol` 给出）
 * @returns 按响应内顺序排列的 content blocks（无法解析时返回空数组，由调用方回退到原文展示）
 */
export function parseResponseBlocks(
  responseBody: string,
  format: ResponseFormat,
  protocol: ApiProtocolName,
): ContentBlockInfo[] {
  try {
    // 1. Responses API
    if (protocol === 'openai_response') {
      return format === 'sse'
        ? parseOpenAIResponsesSSE(responseBody)
        : parseOpenAIResponsesResponse(responseBody);
    }

    // 2. Chat Completions：解析为空时用 Responses 兜底（容忍服务端形态漂移）
    if (protocol === 'openai') {
      const chatBlocks =
        format === 'sse' ? parseOpenAIChatSSE(responseBody) : parseOpenAIChatResponse(responseBody);
      if (chatBlocks.length > 0) return chatBlocks;
      return format === 'sse'
        ? parseOpenAIResponsesSSE(responseBody)
        : parseOpenAIResponsesResponse(responseBody);
    }

    // 3. Anthropic
    return parseStructuredBlocks(responseBody, format).content_blocks;
  } catch {
    // 响应体与探测到的格式不符（如 HTML 错误页被当作 JSON）时不抛给渲染层，
    // 返回空数组让调用方以原文记录兜底
    return [];
  }
}
