//! 词元用量解析器（基础设施层）
//!
//! 从上游响应中解析词元用量信息，支持 4 种协议格式。
//!
//! ## 归一化策略
//!
//! 不同 LLM 服务商对 usage 字段的语义定义不同，本模块负责归一化为统一的互斥语义：
//! 5 个词元维度（input / output / cache_creation / cache_read / thinking）相互独立、可加法求和。
//!
//! | 协议                    | 原始语义                         | 归一化操作                        |
//! | ----------------------- | -------------------------------- | --------------------------------- |
//! | Anthropic               | 5 个字段互斥，天然符合           | 无需处理                          |
//! | OpenAI Chat Completions | `cached_tokens` ⊆ `prompt_tokens` | `input = prompt - cached`         |
//! | OpenAI Responses API    | `reasoning_tokens` ⊆ `output_tokens` | `output = output - reasoning` |

use serde_json::Value;

/// 从响应中解析词元用量
///
/// 按优先级依次尝试：
/// 1. Anthropic SSE（`event:` / `data:` 行 + `message_delta` 事件）
/// 2. Anthropic 非流式（顶层 `usage` 对象，`input_tokens` 字段族）
/// 3. OpenAI Chat Completions（SSE 或非流式，`prompt_tokens` 字段族）
/// 4. OpenAI Responses API（SSE 或非流式，`input_tokens` 字段族 + `output_tokens_details.reasoning_tokens`）
pub(crate) fn parse_usage_from_response(response_body: &str) -> Option<ParsedTokenUsage> {
    // 先尝试 Anthropic SSE 格式
    if let Some(usage) = parse_sse_usage(response_body) {
        return Some(usage);
    }

    // 回退：非流式 JSON 格式（Anthropic）
    if let Some(usage) = parse_non_streaming_usage(response_body) {
        return Some(usage);
    }

    // 尝试 OpenAI Chat Completions 格式
    if let Some(usage) = extract_openai_chat_usage(response_body) {
        return Some(usage);
    }

    // 尝试 OpenAI Responses API 格式
    if let Some(usage) = extract_openai_responses_usage(response_body) {
        return Some(usage);
    }

    None
}

// ─── Anthropic 格式解析 ───

fn parse_sse_usage(response_body: &str) -> Option<ParsedTokenUsage> {
    let mut event_type = None;
    let mut data_lines = Vec::new();

    for line in response_body.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            event_type = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("data:") {
            data_lines.push(value.trim().to_string());
        } else if line.trim().is_empty() && !data_lines.is_empty() {
            let result = process_usage_event(event_type.take(), &mut data_lines);
            if let Some(usage) = result {
                return Some(usage);
            }
        }
    }

    if !data_lines.is_empty() {
        let result = process_usage_event(event_type, &mut data_lines);
        if let Some(usage) = result {
            return Some(usage);
        }
    }

    None
}

/// 非流式 JSON 响应格式，检查顶层 `usage` 字段（Anthropic 命名约定）
///
/// 仅当 `usage` 对象包含 Anthropic 特有的 `input_tokens` 字段时才匹配；
/// 如果包含 `prompt_tokens`（OpenAI Chat Completions），则返回 `None`，以便后续 OpenAI 解析器处理。
fn parse_non_streaming_usage(response_body: &str) -> Option<ParsedTokenUsage> {
    let json: Value = serde_json::from_str(response_body).ok()?;
    let usage = json.get("usage")?;
    // 区分 Anthropic 和 OpenAI 格式：
    // Anthropic 的 usage 对象使用 `input_tokens` 字段，OpenAI 使用 `prompt_tokens`
    if usage.get("prompt_tokens").is_some() || usage.get("output_tokens_details").is_some() {
        return None;
    }
    Some(extract_usage_fields(usage))
}

fn process_usage_event(
    event_type: Option<String>,
    data_lines: &mut Vec<String>,
) -> Option<ParsedTokenUsage> {
    let data = data_lines.join("\n");
    data_lines.clear();

    if data == "[DONE]" || data.is_empty() {
        return None;
    }

    if let Ok(json) = serde_json::from_str::<Value>(&data) {
        let kind = event_type
            .or_else(|| {
                json.get("type")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_default();

        if kind == "message_delta" {
            if let Some(raw_usage) = json.get("usage") {
                return Some(extract_usage_fields(raw_usage));
            }
        }
    }

    None
}

/// 解析 Anthropic 命名约定的 usage 字段
///
/// Anthropic 的 5 个词元维度是互斥的加法维度（input / output / cache_creation / cache_read / thinking），
/// 无需做重叠扣除，直接读取并求和即为总量。
fn extract_usage_fields(raw_usage: &Value) -> ParsedTokenUsage {
    let input = int_field(raw_usage, "input_tokens");
    let output = int_field(raw_usage, "output_tokens");
    let cache_creation = int_field(raw_usage, "cache_creation_input_tokens");
    let cache_read = int_field(raw_usage, "cache_read_input_tokens");
    let thinking = int_field(raw_usage, "thinking_tokens");

    ParsedTokenUsage {
        input_tokens: input,
        output_tokens: output,
        cache_creation_input_tokens: cache_creation,
        cache_read_input_tokens: cache_read,
        thinking_tokens: thinking,
        total_tokens: input + output + cache_creation + cache_read + thinking,
        raw_usage: raw_usage.clone(),
    }
}

// ─── OpenAI Chat Completions 格式解析 ───

/// 尝试从 OpenAI Chat Completions 格式的响应体中提取词元用量
///
/// 支持流式（SSE `data:` 行中最后一个含 `usage` 的 chunk）和非流式（顶层 `usage` 对象）。
fn extract_openai_chat_usage(body: &str) -> Option<ParsedTokenUsage> {
    // 先尝试非流式：解析完整 JSON，查找顶层 `usage` 对象
    if let Ok(root) = serde_json::from_str::<Value>(body) {
        if let Some(usage) = root.get("usage") {
            return parse_openai_usage_fields(usage);
        }
    }

    // 然后尝试 SSE 流式：扫描 `data:` 行，收集最后一个含 `usage` 的 chunk
    let mut last_usage: Option<Value> = None;
    for line in body.lines() {
        let line = line.trim();
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                continue;
            }
            if let Ok(chunk) = serde_json::from_str::<Value>(data) {
                if let Some(usage) = chunk.get("usage").cloned() {
                    last_usage = Some(usage);
                }
            }
        }
    }

    last_usage.and_then(|usage| parse_openai_usage_fields(&usage))
}

/// 解析 OpenAI Chat Completions 的 `usage` 对象字段
///
/// 字段映射（OpenAI Chat Completions 与 Anthropic 语义不同，需归一化）：
/// - `prompt_tokens` **包含** `cached_tokens`，需扣除后存入 `input_tokens`
/// - `completion_tokens` → `output_tokens`
/// - `prompt_tokens_details.cached_tokens` → `cache_read_input_tokens`
/// - `total_tokens` → 直接使用 API 返回的总量
///
/// 归一化后 `input + cache_read = prompt_tokens`（与 Anthropic 的互斥语义对齐）。
fn parse_openai_usage_fields(usage: &Value) -> Option<ParsedTokenUsage> {
    let prompt_tokens = usage
        .get("prompt_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    let completion_tokens = usage
        .get("completion_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    let cached_tokens = usage
        .get("prompt_tokens_details")
        .and_then(|v| v.get("cached_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;

    // OpenAI 的 cached_tokens 是 prompt_tokens 的子集；归一化为互斥语义后
    // input_tokens = 未命中缓存，cache_read = 命中缓存
    let cache_read = cached_tokens;
    let input = (prompt_tokens - cached_tokens).max(0);

    Some(ParsedTokenUsage {
        input_tokens: input,
        output_tokens: completion_tokens,
        cache_creation_input_tokens: 0, // OpenAI Chat Completions 无此概念
        cache_read_input_tokens: cache_read,
        thinking_tokens: 0, // Chat Completions 无 reasoning_tokens
        total_tokens,
        raw_usage: usage.clone(),
    })
}

// ─── OpenAI Responses API 格式解析 ───

/// 尝试从 OpenAI Responses API 格式的响应体中提取词元用量
///
/// 支持非流式（顶层 `usage` 对象）和流式（`response.completed` 事件中的 `response.usage`）。
fn extract_openai_responses_usage(body: &str) -> Option<ParsedTokenUsage> {
    // 先尝试非流式：解析完整 JSON，查找顶层 `usage` 对象
    if let Ok(root) = serde_json::from_str::<Value>(body) {
        if let Some(usage) = root.get("usage") {
            return parse_openai_responses_usage_fields(usage);
        }
    }

    // 然后尝试 SSE 流式：扫描 `data:` 行，查找 `response.completed` 事件
    for line in body.lines() {
        let line = line.trim();
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(event) = serde_json::from_str::<Value>(data) {
                if event.get("type").and_then(Value::as_str) == Some("response.completed") {
                    if let Some(response) = event.get("response") {
                        if let Some(usage) = response.get("usage") {
                            return parse_openai_responses_usage_fields(usage);
                        }
                    }
                }
            }
        }
    }

    None
}

/// 解析 OpenAI Responses API 的 `usage` 对象字段
///
/// 字段映射（OpenAI Responses API 与 Anthropic 语义不同，需归一化）：
/// - `input_tokens` → `input_tokens`（Responses API 无缓存概念，直接使用）
/// - `output_tokens` **包含** `reasoning_tokens`，需扣除后存入 `output_tokens`
/// - `output_tokens_details.reasoning_tokens` → `thinking_tokens`
/// - `total_tokens` → 直接使用 API 返回的总量
///
/// 归一化后 `output + thinking = output_tokens`（与 Anthropic 的互斥语义对齐）。
fn parse_openai_responses_usage_fields(usage: &Value) -> Option<ParsedTokenUsage> {
    let input_tokens = usage
        .get("input_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    let output_tokens_raw = usage
        .get("output_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    let reasoning_tokens = usage
        .get("output_tokens_details")
        .and_then(|v| v.get("reasoning_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;

    // OpenAI 的 reasoning_tokens 是 output_tokens 的子集；归一化为互斥语义后
    // output_tokens = 不含思考的输出，thinking = 推理词元
    let output = (output_tokens_raw - reasoning_tokens).max(0);

    Some(ParsedTokenUsage {
        input_tokens,
        output_tokens: output,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0, // Responses API 暂未有 cached_tokens
        thinking_tokens: reasoning_tokens,
        total_tokens,
        raw_usage: usage.clone(),
    })
}

fn int_field(value: &Value, key: &str) -> i32 {
    value.get(key).and_then(Value::as_i64).unwrap_or_default() as i32
}

// ─── 类型定义 ───

/// 解析后的词元用量
#[derive(Debug, Clone, Default)]
pub struct ParsedTokenUsage {
    /// 输入词元数
    pub input_tokens: i32,
    /// 输出词元数
    pub output_tokens: i32,
    /// 缓存创建输入词元数
    pub cache_creation_input_tokens: i32,
    /// 缓存命中输入词元数
    pub cache_read_input_tokens: i32,
    /// 思考词元数
    pub thinking_tokens: i32,
    /// 总计词元数（各字段之和）
    pub total_tokens: i32,
    /// 原始 usage 数据（JSON）
    pub raw_usage: Value,
}

// ─── 单元测试 ───

#[cfg(test)]
mod tests {
    use super::*;

    // ── Anthropic 格式测试 ──

    #[test]
    fn test_extract_usage_from_sse() {
        let sse_data = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_001","usage":{"input_tokens":10}}}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":25,"input_tokens":0,"cache_creation_input_tokens":5,"cache_read_input_tokens":3}}

"#;

        let result = parse_sse_usage(sse_data).unwrap();
        assert_eq!(result.input_tokens, 0);
        assert_eq!(result.output_tokens, 25);
        assert_eq!(result.cache_creation_input_tokens, 5);
        assert_eq!(result.cache_read_input_tokens, 3);
        assert_eq!(result.total_tokens, 33);
    }

    #[test]
    fn test_parse_non_streaming_usage() {
        let json_data =
            r#"{"id":"msg_123","type":"message","usage":{"input_tokens":100,"output_tokens":200}}"#;

        let result = parse_non_streaming_usage(json_data).unwrap();
        assert_eq!(result.input_tokens, 100);
        assert_eq!(result.output_tokens, 200);
        assert_eq!(result.total_tokens, 300);
    }

    // ── OpenAI Chat Completions 格式测试 ──

    /// 非流式 OpenAI Chat Completions 响应（含 `cached_tokens`）
    #[test]
    fn test_extract_openai_chat_non_streaming() {
        let json_data = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "model": "gpt-4o",
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 200,
                "total_tokens": 300,
                "prompt_tokens_details": {
                    "cached_tokens": 50
                }
            }
        }"#;

        let result = extract_openai_chat_usage(json_data).unwrap();
        // prompt_tokens = 100, cached_tokens = 50 → input = 100 - 50 = 50
        assert_eq!(result.input_tokens, 50);
        assert_eq!(result.output_tokens, 200);
        assert_eq!(result.cache_read_input_tokens, 50);
        assert_eq!(result.cache_creation_input_tokens, 0);
        assert_eq!(result.thinking_tokens, 0);
        assert_eq!(result.total_tokens, 300);
    }

    /// SSE 流式 OpenAI Chat Completions 最后一个 chunk 含 `usage`
    #[test]
    fn test_extract_openai_chat_sse() {
        let sse_data = r#"
data: {"id":"chatcmpl-001","object":"chat.completion.chunk","choices":[{"delta":{"role":"assistant"},"index":0}]}

data: {"id":"chatcmpl-001","object":"chat.completion.chunk","choices":[{"delta":{"content":"Hello"},"index":0}]}

data: {"id":"chatcmpl-001","object":"chat.completion.chunk","choices":[{"delta":{},"finish_reason":"stop","index":0}],"usage":{"prompt_tokens":150,"completion_tokens":300,"total_tokens":450,"prompt_tokens_details":{"cached_tokens":80}}}

data: [DONE]
"#;

        let result = extract_openai_chat_usage(sse_data).unwrap();
        // prompt_tokens = 150, cached_tokens = 80 → input = 150 - 80 = 70
        assert_eq!(result.input_tokens, 70);
        assert_eq!(result.output_tokens, 300);
        assert_eq!(result.cache_read_input_tokens, 80);
        assert_eq!(result.total_tokens, 450);
    }

    // ── OpenAI Responses API 格式测试 ──

    /// 非流式 OpenAI Responses API 响应
    #[test]
    fn test_extract_openai_responses_non_streaming() {
        let json_data = r#"{
            "id": "resp_001",
            "object": "response",
            "model": "gpt-4o",
            "usage": {
                "input_tokens": 200,
                "output_tokens": 500,
                "output_tokens_details": {
                    "reasoning_tokens": 100
                },
                "total_tokens": 700
            }
        }"#;

        let result = extract_openai_responses_usage(json_data).unwrap();
        // output_tokens = 500, reasoning_tokens = 100 → output = 500 - 100 = 400
        assert_eq!(result.input_tokens, 200);
        assert_eq!(result.output_tokens, 400);
        assert_eq!(result.thinking_tokens, 100);
        assert_eq!(result.cache_read_input_tokens, 0);
        assert_eq!(result.cache_creation_input_tokens, 0);
        assert_eq!(result.total_tokens, 700);
    }

    /// SSE 流式 OpenAI Responses API 的 `response.completed` 事件
    #[test]
    fn test_extract_openai_responses_sse() {
        let sse_data = r#"
data: {"type":"response.output_text.delta","delta":"Hi"}

data: {"type":"response.completed","response":{"id":"resp_002","model":"gpt-4o","usage":{"input_tokens":50,"output_tokens":120,"output_tokens_details":{"reasoning_tokens":40},"total_tokens":170}}}

data: [DONE]
"#;

        let result = extract_openai_responses_usage(sse_data).unwrap();
        // output_tokens = 120, reasoning_tokens = 40 → output = 120 - 40 = 80
        assert_eq!(result.input_tokens, 50);
        assert_eq!(result.output_tokens, 80);
        assert_eq!(result.thinking_tokens, 40);
        assert_eq!(result.total_tokens, 170);
    }

    // ── 主函数 fallback 测试 ──

    /// 验证 Anthropic 格式未匹配时自动尝试 OpenAI Chat Completions
    #[test]
    fn test_parse_usage_falls_back_to_openai_chat() {
        let body = r#"{"id":"chatcmpl-fallback","object":"chat.completion","usage":{"prompt_tokens":42,"completion_tokens":84,"total_tokens":126}}"#;

        let result = parse_usage_from_response(body).unwrap();
        assert_eq!(result.input_tokens, 42);
        assert_eq!(result.output_tokens, 84);
        assert_eq!(result.total_tokens, 126);
    }

    /// 验证 Anthropic 格式未匹配时自动尝试 OpenAI Responses API
    #[test]
    fn test_parse_usage_falls_back_to_openai_responses() {
        let body = r#"data: {"type":"response.completed","response":{"id":"r","usage":{"input_tokens":10,"output_tokens":20,"output_tokens_details":{"reasoning_tokens":5},"total_tokens":30}}}"#;

        let result = parse_usage_from_response(body).unwrap();
        // output_tokens = 20, reasoning_tokens = 5 → output = 20 - 5 = 15
        assert_eq!(result.input_tokens, 10);
        assert_eq!(result.output_tokens, 15);
        assert_eq!(result.thinking_tokens, 5);
        assert_eq!(result.total_tokens, 30);
    }

    /// 验证 Anthropic 格式仍然优先于 OpenAI 格式
    #[test]
    fn test_anthropic_still_parses_first() {
        let body = r#"event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"input_tokens":0,"output_tokens":99}}

"#;

        // 这应该被 Anthropic 格式解析，而不是 OpenAI
        let result = parse_usage_from_response(body).unwrap();
        assert_eq!(result.output_tokens, 99);
    }

    /// 验证带 `prompt_tokens_details.cached_tokens` 的非流式响应仍正确解析
    #[test]
    fn test_openai_chat_without_cached_tokens() {
        let json_data = r#"{"id":"ch","object":"chat.completion","usage":{"prompt_tokens":10,"completion_tokens":20,"total_tokens":30}}"#;

        let result = extract_openai_chat_usage(json_data).unwrap();
        assert_eq!(result.input_tokens, 10);
        assert_eq!(result.output_tokens, 20);
        assert_eq!(result.cache_read_input_tokens, 0); // 无 cached_tokens 时默认为 0
        assert_eq!(result.total_tokens, 30);
    }

    /// 验证 Responses API 无 reasoning_tokens 时默认值为 0
    #[test]
    fn test_openai_responses_without_reasoning_tokens() {
        let json_data = r#"{"id":"r","object":"response","usage":{"input_tokens":10,"output_tokens":20,"total_tokens":30}}"#;

        let result = extract_openai_responses_usage(json_data).unwrap();
        assert_eq!(result.input_tokens, 10);
        assert_eq!(result.output_tokens, 20);
        assert_eq!(result.thinking_tokens, 0);
        assert_eq!(result.total_tokens, 30);
    }
}
