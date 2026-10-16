//! 词元用量解析器（基础设施层）
//!
//! 从上游响应中解析词元用量信息，支持 3 条线协议（4 种响应形态）。
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
//!
//! ## 协议感知解析（为什么必须按协议分发）
//!
//! Anthropic 与 OpenAI Responses 的**非流式**响应都使用顶层 `usage.input_tokens`，
//! 结构上无法区分。若只按字段形状探测，Responses 的 `usage` 会被 Anthropic 解析器
//! 先接走，导致 `reasoning_tokens` 扣减规则**静默失效**。
//! 因此入口 [`parse_usage_for_protocol`] 以请求级协议为准直接路由；
//! 仅在对应解析器返回空时才回退到全协议探测链。
//!
//! ## 总词元口径（唯一计算方式）
//!
//! `total_tokens` 一律由 5 个互斥维度求和得出，不使用上游返回的 `total_tokens`：
//! 归一化后输入侧 `input + cache_read` 等于上游 `prompt_tokens`/`input_tokens`，
//! 输出侧 `output + thinking` 等于上游 `completion_tokens`/`output_tokens`，
//! 两种口径数值一致；而求和口径在流式分片缺少总量字段时仍能算出正确结果。

use serde_json::Value;

use crate::domain::shared::ApiProtocol;

/// 按请求级协议解析词元用量
///
/// 协议由代理管道在请求路径上确定（`ApiProtocol`），此处直接路由，
/// 避免 Anthropic 与 OpenAI Responses 的非流式响应被互相误解析。
///
/// 对应解析器返回空时回退 [`parse_usage_from_response`]（全协议探测链），
/// 以容忍上游返回了与请求协议不一致的响应体形态。
pub(crate) fn parse_usage_for_protocol(
    protocol: ApiProtocol,
    response_body: &str,
) -> Option<ParsedTokenUsage> {
    let by_protocol = match protocol {
        ApiProtocol::Anthropic => parse_anthropic_usage(response_body),
        ApiProtocol::OpenAi => extract_openai_chat_usage(response_body),
        ApiProtocol::OpenAiResponse => extract_openai_responses_usage(response_body),
    };

    by_protocol.or_else(|| parse_usage_from_response(response_body))
}

/// 从响应中解析词元用量（全协议探测链，无法确定协议时的兜底）
///
/// 按优先级依次尝试：
/// 1. Anthropic SSE（`message_start` + `message_delta` 事件，usage 按维度合并）
/// 2. Anthropic 非流式（顶层 `usage` 对象，`input_tokens` 字段族）
/// 3. OpenAI Chat Completions（SSE 或非流式，`prompt_tokens` 字段族）
/// 4. OpenAI Responses API（`response.completed` 事件或顶层 `usage` + `output_tokens_details`）
///
/// 注意：第 2 步会吞掉 Responses 的非流式响应。已知协议时应优先调用
/// [`parse_usage_for_protocol`]。
pub(crate) fn parse_usage_from_response(response_body: &str) -> Option<ParsedTokenUsage> {
    // 先尝试 Anthropic SSE 格式
    if let Some(usage) = parse_anthropic_sse_usage(response_body) {
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

/// Anthropic 协议解析链：SSE 事件合并优先，其次非流式顶层 `usage`
fn parse_anthropic_usage(response_body: &str) -> Option<ParsedTokenUsage> {
    parse_anthropic_sse_usage(response_body).or_else(|| parse_non_streaming_usage(response_body))
}

// ─── Anthropic 格式解析 ───

/// 解析 Anthropic 流式（SSE）响应中的词元用量
///
/// Anthropic 把 usage **拆在两个事件**里，只取其一必然漏计：
/// - `message_start.message.usage`：输入侧（`input_tokens` / `cache_creation_input_tokens` /
///   `cache_read_input_tokens`），`output_tokens` 只是起步占位值
/// - `message_delta.usage`：输出侧最终 `output_tokens`
///
/// 因此逐个事件按维度合并：同名维度取最大值（`message_start` 提供输入侧、
/// `message_delta` 提供最终输出侧），`raw_usage` 把两个 usage 对象浅合并后保存。
fn parse_anthropic_sse_usage(response_body: &str) -> Option<ParsedTokenUsage> {
    let mut merged: Option<ParsedTokenUsage> = None;

    for (event_type, data) in iter_sse_events(response_body) {
        let Ok(json) = serde_json::from_str::<Value>(&data) else {
            continue;
        };

        let kind = event_type
            .or_else(|| {
                json.get("type")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_default();

        // 1. message_start 的 usage 嵌在 message 下；message_delta 的 usage 在顶层
        let raw_usage = match kind.as_str() {
            "message_start" => json.get("message").and_then(|message| message.get("usage")),
            "message_delta" => json.get("usage"),
            _ => continue,
        };
        let Some(raw_usage) = raw_usage else {
            continue;
        };

        // 2. 首个 usage 初始化，后续按维度归并
        match merged.as_mut() {
            Some(acc) => merge_anthropic_usage(acc, raw_usage),
            None => merged = Some(extract_usage_fields(raw_usage)),
        }
    }

    merged
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

/// 把后到的 Anthropic usage 事件归并进累积结果
///
/// 各维度取最大值：同一个维度的最终值只会出现在其中一侧
/// （输入侧在 `message_start`、输出侧在 `message_delta`），取最大值即可规避
/// `message_start` 的 `output_tokens` 起步占位值（通常为 1）污染最终输出。
fn merge_anthropic_usage(acc: &mut ParsedTokenUsage, raw_usage: &Value) {
    acc.input_tokens = acc.input_tokens.max(int_field(raw_usage, "input_tokens"));
    acc.output_tokens = acc.output_tokens.max(int_field(raw_usage, "output_tokens"));
    acc.cache_creation_input_tokens = acc
        .cache_creation_input_tokens
        .max(int_field(raw_usage, "cache_creation_input_tokens"));
    acc.cache_read_input_tokens = acc
        .cache_read_input_tokens
        .max(int_field(raw_usage, "cache_read_input_tokens"));
    acc.thinking_tokens = acc
        .thinking_tokens
        .max(int_field(raw_usage, "thinking_tokens"));

    // 原始 usage 浅合并：后出现的事件覆盖同名字段（message_delta 的 output_tokens 为最终值）
    if let (Some(acc_map), Some(raw_map)) = (acc.raw_usage.as_object_mut(), raw_usage.as_object()) {
        for (key, value) in raw_map {
            acc_map.insert(key.clone(), value.clone());
        }
    }

    acc.total_tokens = sum_dimensions(acc);
}

/// 解析 Anthropic 命名约定的 usage 字段
///
/// Anthropic 的 5 个词元维度是互斥的加法维度（input / output / cache_creation / cache_read / thinking），
/// 无需做重叠扣除，直接读取并求和即为总量。
fn extract_usage_fields(raw_usage: &Value) -> ParsedTokenUsage {
    let mut usage = ParsedTokenUsage {
        input_tokens: int_field(raw_usage, "input_tokens"),
        output_tokens: int_field(raw_usage, "output_tokens"),
        cache_creation_input_tokens: int_field(raw_usage, "cache_creation_input_tokens"),
        cache_read_input_tokens: int_field(raw_usage, "cache_read_input_tokens"),
        thinking_tokens: int_field(raw_usage, "thinking_tokens"),
        total_tokens: 0,
        raw_usage: raw_usage.clone(),
    };
    usage.total_tokens = sum_dimensions(&usage);
    usage
}

// ─── SSE 分帧 ───

/// 按 SSE 规范切分响应体为 `(事件类型, data 负载)` 列表
///
/// 兼容两类不规范但常见的分帧：
/// - `data:{...}`（冒号后不带空格）
/// - 事件之间缺少空行（靠新的 `event:`/`data:` 行隐式断开）
///
/// 多行 `data:` 负载按规范以换行拼接；`[DONE]` 哨兵与空负载不产出事件。
fn iter_sse_events(body: &str) -> Vec<(Option<String>, String)> {
    let mut events = Vec::new();
    let mut event_type: Option<String> = None;
    let mut data_lines: Vec<String> = Vec::new();

    for line in body.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            // 1. 新事件类型出现即收束上一个事件（兼容缺少空行的分帧）
            push_sse_event(&mut events, &mut event_type, &mut data_lines);
            event_type = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("data:") {
            // 2. 已有负载若是完整 JSON，说明上一个事件已结束
            if !data_lines.is_empty() && is_complete_json(&data_lines.join("\n")) {
                push_sse_event(&mut events, &mut event_type, &mut data_lines);
            }
            data_lines.push(value.trim_start().to_string());
        } else if line.trim().is_empty() {
            push_sse_event(&mut events, &mut event_type, &mut data_lines);
        }
    }
    push_sse_event(&mut events, &mut event_type, &mut data_lines);

    events
}

/// 负载是否已是完整 JSON（用于判断事件边界，不做类型解释）
fn is_complete_json(data: &str) -> bool {
    serde_json::from_str::<Value>(data).is_ok()
}

/// 收束一个 SSE 事件：无负载时不产出，`[DONE]` 直接丢弃
fn push_sse_event(
    events: &mut Vec<(Option<String>, String)>,
    event_type: &mut Option<String>,
    data_lines: &mut Vec<String>,
) {
    let data = data_lines.join("\n");
    data_lines.clear();
    let kind = event_type.take();

    if data.is_empty() || data == "[DONE]" {
        return;
    }
    events.push((kind, data));
}

// ─── OpenAI Chat Completions 格式解析 ───

/// 尝试从 OpenAI Chat Completions 格式的响应体中提取词元用量
///
/// 支持流式（SSE `data:` 行中最后一个含 `usage` 的 chunk）和非流式（顶层 `usage` 对象）。
fn extract_openai_chat_usage(body: &str) -> Option<ParsedTokenUsage> {
    // 先尝试非流式：解析完整 JSON，查找顶层 `usage` 对象
    if let Ok(root) = serde_json::from_str::<Value>(body) {
        if let Some(usage) = root.get("usage").filter(|usage| usage.is_object()) {
            return parse_openai_usage_fields(usage);
        }
    }

    // 然后尝试 SSE 流式：收集最后一个**非空** usage 对象（中间 chunk 的 `usage` 常为 null）
    let mut last_usage: Option<Value> = None;
    for (_, data) in iter_sse_events(body) {
        if let Ok(chunk) = serde_json::from_str::<Value>(&data) {
            if let Some(usage) = chunk.get("usage").filter(|usage| usage.is_object()) {
                last_usage = Some(usage.clone());
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
/// - `total_tokens` 不直接采用，统一由各维度求和（见模块头「总词元口径」）
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
    let cached_tokens = usage
        .get("prompt_tokens_details")
        .and_then(|v| v.get("cached_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;

    // OpenAI 的 cached_tokens 是 prompt_tokens 的子集；归一化为互斥语义后
    // input_tokens = 未命中缓存，cache_read = 命中缓存
    let cache_read = cached_tokens;
    let input = (prompt_tokens - cached_tokens).max(0);

    let mut parsed = ParsedTokenUsage {
        input_tokens: input,
        output_tokens: completion_tokens,
        cache_creation_input_tokens: 0, // OpenAI Chat Completions 无此概念
        cache_read_input_tokens: cache_read,
        thinking_tokens: 0, // Chat Completions 无 reasoning_tokens
        total_tokens: 0,
        raw_usage: usage.clone(),
    };
    parsed.total_tokens = sum_dimensions(&parsed);

    Some(parsed)
}

// ─── OpenAI Responses API 格式解析 ───

/// 尝试从 OpenAI Responses API 格式的响应体中提取词元用量
///
/// 支持非流式（顶层 `usage` 对象）和流式（`response.completed` 事件中的 `response.usage`）。
fn extract_openai_responses_usage(body: &str) -> Option<ParsedTokenUsage> {
    // 先尝试非流式：解析完整 JSON，查找顶层 `usage` 对象
    if let Ok(root) = serde_json::from_str::<Value>(body) {
        if let Some(usage) = root.get("usage").filter(|usage| usage.is_object()) {
            return parse_openai_responses_usage_fields(usage);
        }
    }

    // 然后尝试 SSE 流式：查找 `response.completed` 事件中的 `response.usage`
    for (_, data) in iter_sse_events(body) {
        if let Ok(event) = serde_json::from_str::<Value>(&data) {
            if event.get("type").and_then(Value::as_str) == Some("response.completed") {
                if let Some(usage) = event
                    .get("response")
                    .and_then(|response| response.get("usage"))
                    .filter(|usage| usage.is_object())
                {
                    return parse_openai_responses_usage_fields(usage);
                }
            }
        }
    }

    None
}

/// 解析 OpenAI Responses API 的 `usage` 对象字段
///
/// 优先走成熟 SDK（`async-openai`）的类型化解析：SDK 的 `ResponseUsage`
/// 完整建模了 `input_tokens_details.cached_tokens` 与
/// `output_tokens_details.reasoning_tokens` 两个细节字段，
/// 能把缓存命中输入与思考词元正确拆出来。
///
/// 字段映射（Responses API 与 Anthropic 语义不同，需归一化）：
/// - `input_tokens`（含缓存命中）→ 扣除 `cached_tokens` 后存入 `input_tokens`
/// - `input_tokens_details.cached_tokens` → `cache_read_input_tokens`
/// - `output_tokens`（含 reasoning）→ 扣除 `reasoning_tokens` 后存入 `output_tokens`
/// - `output_tokens_details.reasoning_tokens` → `thinking_tokens`
/// - `total_tokens` 不直接采用，统一由各维度求和（见模块头「总词元口径」）
///
/// 归一化后 `output + thinking = 原始 output_tokens`，
/// `input + cache_read = 原始 input_tokens`（与 Anthropic 的互斥语义对齐）。
///
/// SDK 反序列化失败（如上游返回了 SDK 尚未建模的字段组合）时回退到
/// 原始 JSON 字段提取，保证日志能力不因 SDK 版本滞后而退化。
fn parse_openai_responses_usage_fields(usage: &Value) -> Option<ParsedTokenUsage> {
    match serde_json::from_value::<async_openai::types::responses::ResponseUsage>(usage.clone()) {
        Ok(typed) => Some(normalize_sdk_response_usage(&typed, usage)),
        Err(e) => {
            tracing::debug!(
                error = %e,
                "Responses usage 未通过 SDK 类型化解析，回退原始 JSON 提取"
            );
            parse_openai_responses_usage_fallback(usage)
        }
    }
}

/// 把 SDK 的 `ResponseUsage` 归一化为互斥语义的 [`ParsedTokenUsage`]
fn normalize_sdk_response_usage(
    typed: &async_openai::types::responses::ResponseUsage,
    raw: &Value,
) -> ParsedTokenUsage {
    let cached = typed.input_tokens_details.cached_tokens as i32;
    let reasoning = typed.output_tokens_details.reasoning_tokens as i32;

    // 1. 输入扣除缓存命中部分（缓存命中单独计列）
    let input = (typed.input_tokens as i32 - cached).max(0);
    // 2. 输出扣除推理词元（推理词元单独计列）
    let output = (typed.output_tokens as i32 - reasoning).max(0);

    ParsedTokenUsage {
        input_tokens: input,
        output_tokens: output,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: cached,
        thinking_tokens: reasoning,
        total_tokens: 0,
        raw_usage: raw.clone(),
    }
    .with_computed_total()
}

/// 原始 JSON 字段提取（SDK 类型化解析失败时的兜底）
///
/// 与 [`normalize_sdk_response_usage`] 语义一致，仅字段获取方式不同。
fn parse_openai_responses_usage_fallback(usage: &Value) -> Option<ParsedTokenUsage> {
    let input_tokens_raw = usage
        .get("input_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    let output_tokens_raw = usage
        .get("output_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    let reasoning_tokens = usage
        .get("output_tokens_details")
        .and_then(|v| v.get("reasoning_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    let cached_tokens = usage
        .get("input_tokens_details")
        .and_then(|v| v.get("cached_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;

    // OpenAI 的 reasoning_tokens 是 output_tokens 的子集、cached_tokens 是 input_tokens 的子集；
    // 归一化为互斥语义后各自独立计列
    let input = (input_tokens_raw - cached_tokens).max(0);
    let output = (output_tokens_raw - reasoning_tokens).max(0);

    Some(
        ParsedTokenUsage {
            input_tokens: input,
            output_tokens: output,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: cached_tokens,
            thinking_tokens: reasoning_tokens,
            total_tokens: 0,
            raw_usage: usage.clone(),
        }
        .with_computed_total(),
    )
}

fn int_field(value: &Value, key: &str) -> i32 {
    value.get(key).and_then(Value::as_i64).unwrap_or_default() as i32
}

/// 5 个互斥维度的总量（唯一计算口径，见模块头「总词元口径」）
fn sum_dimensions(usage: &ParsedTokenUsage) -> i32 {
    usage.input_tokens
        + usage.output_tokens
        + usage.cache_creation_input_tokens
        + usage.cache_read_input_tokens
        + usage.thinking_tokens
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
    /// 总计词元数（5 个互斥维度之和，由 [`sum_dimensions`] 计算）
    pub total_tokens: i32,
    /// 原始 usage 数据（JSON）
    pub raw_usage: Value,
}

impl ParsedTokenUsage {
    /// 以 5 个互斥维度之和回填 `total_tokens`（统一总量口径）
    fn with_computed_total(mut self) -> Self {
        self.total_tokens = sum_dimensions(&self);
        self
    }
}

// ─── 单元测试 ───

#[cfg(test)]
mod tests {
    use super::*;

    // ── Anthropic 格式测试 ──

    /// 回归：Anthropic 流式把 usage 拆在 `message_start.message.usage`（输入侧）
    /// 与 `message_delta.usage`（输出侧）。只取 `message_delta` 会把输入/缓存词元
    /// 全部记 0（Claude Code 等流式客户端因此长期只统计到输出词元）。
    #[test]
    fn test_anthropic_streaming_merges_message_start_and_delta() {
        // 真实 Claude Code 流式分帧：message_start 只给输入侧，message_delta 只给最终输出
        let sse_data = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_001","type":"message","role":"assistant","content":[],"usage":{"input_tokens":1208,"cache_creation_input_tokens":96,"cache_read_input_tokens":4096,"output_tokens":1}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":139}}

event: message_stop
data: {"type":"message_stop"}

"#;

        let result = parse_anthropic_sse_usage(sse_data).unwrap();
        // 输入侧来自 message_start
        assert_eq!(result.input_tokens, 1208);
        assert_eq!(result.cache_creation_input_tokens, 96);
        assert_eq!(result.cache_read_input_tokens, 4096);
        // 输出侧取 message_delta 的最终值，而非 message_start 的起步占位值 1
        assert_eq!(result.output_tokens, 139);
        assert_eq!(result.total_tokens, 1208 + 96 + 4096 + 139);
        // raw_usage 保留两侧字段，便于前端溯源
        assert_eq!(result.raw_usage.get("input_tokens").unwrap(), 1208);
        assert_eq!(result.raw_usage.get("output_tokens").unwrap(), 139);
    }

    /// 同一维度的数值在多个事件中重复出现时取最大值（兼容上游重复回传）
    #[test]
    fn test_anthropic_streaming_keeps_max_per_dimension() {
        let sse_data = r#"event: message_start
data: {"type":"message_start","message":{"usage":{"input_tokens":10,"output_tokens":1}}}

event: message_delta
data: {"type":"message_delta","usage":{"output_tokens":25,"input_tokens":0,"cache_creation_input_tokens":5,"cache_read_input_tokens":3}}

"#;

        let result = parse_anthropic_sse_usage(sse_data).unwrap();
        assert_eq!(result.input_tokens, 10);
        assert_eq!(result.output_tokens, 25);
        assert_eq!(result.cache_creation_input_tokens, 5);
        assert_eq!(result.cache_read_input_tokens, 3);
        assert_eq!(result.total_tokens, 43);
    }

    /// 事件类型缺失时以 JSON 内的 `type` 兜底（部分上游不写 `event:` 行）
    #[test]
    fn test_anthropic_streaming_falls_back_to_json_type() {
        let sse_data = r#"data: {"type":"message_start","message":{"usage":{"input_tokens":7}}}

data: {"type":"message_delta","usage":{"output_tokens":9}}

"#;

        let result = parse_anthropic_sse_usage(sse_data).unwrap();
        assert_eq!(result.input_tokens, 7);
        assert_eq!(result.output_tokens, 9);
        assert_eq!(result.total_tokens, 16);
    }

    /// 没有 message_delta 的中断流式：仍保留 message_start 的输入侧词元
    #[test]
    fn test_anthropic_streaming_without_delta_keeps_input() {
        let sse_data = r#"event: message_start
data: {"type":"message_start","message":{"usage":{"input_tokens":321,"cache_read_input_tokens":100,"output_tokens":1}}}

"#;

        let result = parse_anthropic_sse_usage(sse_data).unwrap();
        assert_eq!(result.input_tokens, 321);
        assert_eq!(result.cache_read_input_tokens, 100);
        assert_eq!(result.output_tokens, 1);
        assert_eq!(result.total_tokens, 422);
    }

    /// 非 Anthropic 事件不带 usage，解析器必须返回 None 以便回退到其它协议解析器
    #[test]
    fn test_anthropic_streaming_ignores_non_usage_events() {
        let sse_data = r#"data: {"type":"response.output_text.delta","delta":"Hi"}

data: [DONE]
"#;

        assert!(parse_anthropic_sse_usage(sse_data).is_none());
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

    /// SDK 类型化路径：完整 details 形状应拆出缓存命中输入与思考词元
    #[test]
    fn test_responses_usage_sdk_typed_path_splits_cached_and_reasoning() {
        let usage: Value = serde_json::from_str(
            r#"{
                "input_tokens": 1000,
                "input_tokens_details": { "cached_tokens": 400 },
                "output_tokens": 200,
                "output_tokens_details": { "reasoning_tokens": 50 },
                "total_tokens": 1200
            }"#,
        )
        .unwrap();

        let result = parse_openai_responses_usage_fields(&usage).unwrap();
        // 输入扣除缓存命中：1000 - 400
        assert_eq!(result.input_tokens, 600);
        assert_eq!(result.cache_read_input_tokens, 400);
        // 输出扣除推理：200 - 50
        assert_eq!(result.output_tokens, 150);
        assert_eq!(result.thinking_tokens, 50);
        assert_eq!(result.total_tokens, 1200);
    }

    /// SDK 类型化路径失败（缺 details）时回退原始 JSON 提取，不丢数据
    #[test]
    fn test_responses_usage_falls_back_when_details_missing() {
        let usage: Value = serde_json::from_str(
            r#"{"input_tokens":10,"output_tokens":20,"output_tokens_details":{"reasoning_tokens":5},"total_tokens":30}"#,
        )
        .unwrap();

        let result = parse_openai_responses_usage_fields(&usage).unwrap();
        assert_eq!(result.input_tokens, 10);
        assert_eq!(result.output_tokens, 15);
        assert_eq!(result.thinking_tokens, 5);
        assert_eq!(result.cache_read_input_tokens, 0);
        assert_eq!(result.total_tokens, 30);
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

    // ── 协议感知路由测试 ──

    /// 回归：Responses **非流式**响应使用顶层 `usage.input_tokens`，
    /// 与 Anthropic 形状无法区分。探测链会把它交给后续解析器并读错字段族
    /// （`prompt_tokens`/`completion_tokens` 缺失 → 全部归 0），
    /// 使 `reasoning_tokens` 扣减与 `cached_tokens` 拆分**静默失效**。
    /// 按协议路由后必须走 Responses 解析器拿到正确数值。
    #[test]
    fn test_protocol_routing_responses_non_streaming_splits_reasoning() {
        let body = r#"{"id":"r","object":"response","usage":{
            "input_tokens":100,
            "input_tokens_details":{"cached_tokens":40},
            "output_tokens":50,
            "output_tokens_details":{"reasoning_tokens":20},
            "total_tokens":150
        }}"#;

        // 对照：探测链读不到 prompt_tokens/completion_tokens，词元被静默归 0
        let guessed = parse_usage_from_response(body).unwrap();
        assert_eq!(guessed.input_tokens, 0);
        assert_eq!(guessed.output_tokens, 0);
        assert_eq!(guessed.thinking_tokens, 0);
        assert_eq!(guessed.cache_read_input_tokens, 0);

        // 按协议路由：走 Responses 解析器，正确拆出思考词元与缓存命中
        let routed = parse_usage_for_protocol(ApiProtocol::OpenAiResponse, body).unwrap();
        assert_eq!(routed.input_tokens, 60);
        assert_eq!(routed.cache_read_input_tokens, 40);
        assert_eq!(routed.output_tokens, 30);
        assert_eq!(routed.thinking_tokens, 20);
        assert_eq!(routed.total_tokens, 150);
    }

    /// 按协议路由对 Anthropic 响应仍走 Anthropic 解析链
    #[test]
    fn test_protocol_routing_anthropic_non_streaming() {
        let body = r#"{"usage":{"input_tokens":7,"output_tokens":9,"cache_read_input_tokens":3,"total_tokens":19}}"#;

        let routed = parse_usage_for_protocol(ApiProtocol::Anthropic, body).unwrap();
        assert_eq!(routed.input_tokens, 7);
        assert_eq!(routed.output_tokens, 9);
        assert_eq!(routed.cache_read_input_tokens, 3);
    }

    /// 按协议路由对 Chat Completions 响应走 Chat 解析器
    #[test]
    fn test_protocol_routing_chat_completions() {
        let body = r#"{"id":"c","object":"chat.completion","usage":{"prompt_tokens":100,"completion_tokens":20,"total_tokens":120,"prompt_tokens_details":{"cached_tokens":30}}}"#;

        let routed = parse_usage_for_protocol(ApiProtocol::OpenAi, body).unwrap();
        assert_eq!(routed.input_tokens, 70);
        assert_eq!(routed.cache_read_input_tokens, 30);
        assert_eq!(routed.output_tokens, 20);
        assert_eq!(routed.total_tokens, 120);
    }

    /// 协议解析器返回空时回退全协议探测链（容忍上游响应形态与请求协议不符）
    #[test]
    fn test_protocol_routing_falls_back_to_probe_chain() {
        // 请求按 Responses 协议发出，上游却回了 Anthropic 形态
        let body = r#"{"usage":{"input_tokens":5,"output_tokens":6,"total_tokens":11}}"#;

        let routed = parse_usage_for_protocol(ApiProtocol::OpenAiResponse, body).unwrap();
        assert_eq!(routed.input_tokens, 5);
        assert_eq!(routed.output_tokens, 6);
        assert_eq!(routed.total_tokens, 11);
    }

    // ── 总数口径与 SSE 分帧健壮性 ──

    /// 总词元统一由 5 维求和：上游漏报 `total_tokens` 时不再归零
    #[test]
    fn test_total_tokens_computed_when_upstream_omits_it() {
        let body = r#"{"id":"c","object":"chat.completion","usage":{"prompt_tokens":100,"completion_tokens":20}}"#;

        let parsed = extract_openai_chat_usage(body).unwrap();
        assert_eq!(parsed.total_tokens, 120);
    }

    /// `data:` 后不带空格的分帧同样能被解析（部分上游/代理如此输出）
    #[test]
    fn test_sse_without_space_after_data_colon() {
        let sse_data = r#"data:{"id":"c","object":"chat.completion.chunk","choices":[]}
data:{"id":"c","object":"chat.completion.chunk","choices":[],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}
data:[DONE]
"#;

        let parsed = extract_openai_chat_usage(sse_data).unwrap();
        assert_eq!(parsed.input_tokens, 10);
        assert_eq!(parsed.output_tokens, 5);
        assert_eq!(parsed.total_tokens, 15);
    }

    /// 事件之间缺少空行时分帧不错位（靠 `event:` 行隐式断开）
    #[test]
    fn test_sse_without_blank_line_between_events() {
        let sse_data = r#"event: message_start
data: {"type":"message_start","message":{"usage":{"input_tokens":11,"cache_read_input_tokens":22}}}
event: message_delta
data: {"type":"message_delta","usage":{"output_tokens":33}}
"#;

        let parsed = parse_anthropic_sse_usage(sse_data).unwrap();
        assert_eq!(parsed.input_tokens, 11);
        assert_eq!(parsed.cache_read_input_tokens, 22);
        assert_eq!(parsed.output_tokens, 33);
        assert_eq!(parsed.total_tokens, 66);
    }

    /// 中间 chunk 的 `"usage": null` 不得覆盖最终 chunk 的真实 usage
    #[test]
    fn test_openai_chat_sse_null_usage_does_not_override() {
        let sse_data = r#"data: {"id":"c","object":"chat.completion.chunk","choices":[{"delta":{"content":"hi"}}],"usage":null}

data: {"id":"c","object":"chat.completion.chunk","choices":[],"usage":{"prompt_tokens":30,"completion_tokens":7,"total_tokens":37}}

data: {"id":"c","object":"chat.completion.chunk","choices":[],"usage":null}

data: [DONE]
"#;

        let parsed = extract_openai_chat_usage(sse_data).unwrap();
        assert_eq!(parsed.input_tokens, 30);
        assert_eq!(parsed.output_tokens, 7);
        assert_eq!(parsed.total_tokens, 37);
    }

    /// 全程无 usage 时必须返回 None，让协议路由回退到探测链而不是写入全 0
    #[test]
    fn test_openai_chat_sse_without_usage_returns_none() {
        let sse_data = r#"data: {"id":"c","object":"chat.completion.chunk","choices":[{"delta":{"content":"hi"}}],"usage":null}

data: [DONE]
"#;

        assert!(extract_openai_chat_usage(sse_data).is_none());
    }
}
