use serde_json::Value;

use super::anthropic::AnthropicLogParser;
use crate::domain::services::LogParser;

#[derive(Debug, Clone, Default)]
pub struct ParsedLogContent {
    pub message_preview: Option<String>,
    pub message_full: Option<String>,
    pub response_preview: Option<String>,
    pub request_kind: Option<String>,
    pub primary_tool_name: Option<String>,
    pub has_thinking: bool,
    pub has_tool_use: bool,
    pub events: Vec<ParsedConversationEvent>,
    pub usage: Option<ParsedTokenUsage>,
    pub agent_type: Option<String>,
    pub parent_agent_tool_use_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedConversationEvent {
    pub role: String,
    pub event_type: String,
    pub tool_use_id: Option<String>,
    pub tool_name: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub thinking_content: Option<String>,
    pub display_payload: Option<Value>,
    pub confidence: i16,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedTokenUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_creation_input_tokens: i32,
    pub cache_read_input_tokens: i32,
    pub thinking_tokens: i32,
    pub total_tokens: i32,
    pub raw_usage: Value,
}

/// 解析 Anthropic Messages API 的请求体和响应体
///
/// 内部委托给 `AnthropicLogParser`，然后将领域层结果转换为
/// 下层兼容的 `ParsedLogContent` 类型。
pub async fn parse(
    request_body: &Value,
    response_body: &str,
) -> ParsedLogContent {
    let parser = AnthropicLogParser::new();
    let domain_result = parser
        .parse(request_body, response_body)
        .await
        // 解析失败时返回空结果，确保不中断日志记录流程
        .unwrap_or_else(|_| crate::domain::services::ParsedLogContent {
            parser_version: "1.0.0".to_string(),
            message_preview: None,
            message_full: None,
            request_kind: "messages".to_string(),
            primary_tool_name: None,
            response_preview: None,
            response_assistant_text: None,
            content_blocks: Vec::new(),
            thinking_content: None,
            has_thinking: false,
            has_tool_use: false,
        });

    // 从 domain 结果转换为兼容结构
    let events = build_events_from_domain(&domain_result, request_body);
    let usage = parse_usage_from_response(response_body);

    // 从 content_blocks 中提取 agent 信息
    let mut agent_type = None;
    let mut parent_agent_tool_use_id = None;
    for block in &domain_result.content_blocks {
        if block.block_type == "tool_use"
            && block.tool_name.as_deref() == Some("Agent")
        {
            parent_agent_tool_use_id = block.tool_use_id.clone();
            // Agent 类型的 tool_use 没有额外结构，agent_type 从响应体中提取
            // 保留现有行为：从 usage 解析或从 SSE 原始数据提取
            break;
        }
    }

    // 尝试从原始 SSE 解析 agent_type 和 parent_agent_tool_use_id
    // 保持与旧解析器相同的行为
    let (agent_type_found, parent_tool_use_id_found) =
        extract_agent_info_from_sse(response_body);
    if agent_type_found.is_some() {
        agent_type = agent_type_found;
    }
    if parent_tool_use_id_found.is_some() {
        parent_agent_tool_use_id = parent_tool_use_id_found;
    }

    ParsedLogContent {
        message_preview: domain_result.message_preview,
        message_full: domain_result.message_full,
        response_preview: domain_result.response_preview,
        request_kind: Some(domain_result.request_kind),
        primary_tool_name: domain_result.primary_tool_name,
        has_thinking: domain_result.has_thinking,
        has_tool_use: domain_result.has_tool_use,
        events,
        usage,
        agent_type,
        parent_agent_tool_use_id,
    }
}

/// 从 domain 层的 ParsedLogContent 和请求体构建兼容的 events 列表
pub(crate) fn build_events_from_domain(
    domain: &crate::domain::services::ParsedLogContent,
    _request_body: &Value,
) -> Vec<ParsedConversationEvent> {
    let mut events = Vec::new();

    // 添加用户消息事件
    if let Some(text) = &domain.message_full {
        events.push(ParsedConversationEvent {
            role: "user".to_string(),
            event_type: "user_message".to_string(),
            tool_use_id: None,
            tool_name: None,
            title: None,
            content: Some(text.clone()),
            thinking_content: None,
            display_payload: None,
            confidence: 100,
        });
    }

    // 从 content_blocks 转换为 assistant 事件
    for block in &domain.content_blocks {
        match block.block_type.as_str() {
            "thinking" => {
                if let Some(thinking_content) = &block.thinking_content {
                    if !thinking_content.is_empty() {
                        events.push(ParsedConversationEvent {
                            role: "assistant".to_string(),
                            event_type: "assistant_thinking".to_string(),
                            tool_use_id: None,
                            tool_name: None,
                            title: Some("思考过程".to_string()),
                            content: None,
                            thinking_content: Some(thinking_content.clone()),
                            display_payload: None,
                            confidence: 100,
                        });
                    }
                }
            }
            "tool_use" => {
                let tool_name = block.tool_name.clone().unwrap_or_default();
                let tool_id = block.tool_use_id.clone().unwrap_or_default();
                let is_agent = tool_name == "Agent";

                events.push(ParsedConversationEvent {
                    role: "assistant".to_string(),
                    event_type: if is_agent {
                        "agent_call".to_string()
                    } else {
                        "tool_use".to_string()
                    },
                    tool_use_id: Some(tool_id.clone()),
                    tool_name: Some(tool_name.clone()),
                    title: if is_agent {
                        Some(format!("启动子代理：{}", tool_name))
                    } else {
                        Some(format!("工具调用：{}", tool_name))
                    },
                    content: None,
                    thinking_content: None,
                    display_payload: None,
                    confidence: 100,
                });
            }
            "text" => {
                if let Some(content) = &block.content {
                    if !content.is_empty() {
                        events.push(ParsedConversationEvent {
                            role: "assistant".to_string(),
                            event_type: "assistant_message".to_string(),
                            tool_use_id: None,
                            tool_name: None,
                            title: None,
                            content: Some(content.clone()),
                            thinking_content: None,
                            display_payload: None,
                            confidence: 100,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    events
}

/// 从 SSE 响应中解析 token 用量
pub(crate) fn parse_usage_from_response(response_body: &str) -> Option<ParsedTokenUsage> {
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

/// 处理单个 SSE event，提取 usage 信息
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
            .or_else(|| json.get("type").and_then(Value::as_str).map(ToOwned::to_owned))
            .unwrap_or_default();

        if kind == "message_delta" {
            if let Some(raw_usage) = json.get("usage") {
                let input = int_field(&raw_usage, "input_tokens");
                let output = int_field(&raw_usage, "output_tokens");
                let cache_creation = int_field(&raw_usage, "cache_creation_input_tokens");
                let cache_read = int_field(&raw_usage, "cache_read_input_tokens");
                let thinking = int_field(&raw_usage, "thinking_tokens");

                return Some(ParsedTokenUsage {
                    input_tokens: input,
                    output_tokens: output,
                    cache_creation_input_tokens: cache_creation,
                    cache_read_input_tokens: cache_read,
                    thinking_tokens: thinking,
                    total_tokens: input + output + cache_creation + cache_read + thinking,
                    raw_usage: raw_usage.clone(),
                });
            }
        }
    }

    None
}

/// 从 SSE 响应中提取 agent 信息
///
/// 解析 content_block_start 事件中的 tool_use block，
/// 提取 Agent 类型的 tool_use_id 和 subagent_type。
pub(crate) fn extract_agent_info_from_sse(
    response_body: &str,
) -> (Option<String>, Option<String>) {
    let mut event_type = None;
    let mut data_lines = Vec::new();
    let mut agent_type: Option<String> = None;
    let mut parent_tool_use_id: Option<String> = None;

    for line in response_body.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            event_type = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("data:") {
            data_lines.push(value.trim().to_string());
        } else if line.trim().is_empty() && !data_lines.is_empty() {
            check_agent_event(
                event_type.take(),
                &mut data_lines,
                &mut agent_type,
                &mut parent_tool_use_id,
            );
            if agent_type.is_some() {
                return (agent_type, parent_tool_use_id);
            }
        }
    }

    if !data_lines.is_empty() {
        check_agent_event(
            event_type,
            &mut data_lines,
            &mut agent_type,
            &mut parent_tool_use_id,
        );
    }

    (agent_type, parent_tool_use_id)
}

/// 检查单个 SSE event 是否为 Agent tool_use
fn check_agent_event(
    event_type: Option<String>,
    data_lines: &mut Vec<String>,
    agent_type: &mut Option<String>,
    parent_tool_use_id: &mut Option<String>,
) {
    let data = data_lines.join("\n");
    data_lines.clear();

    if data == "[DONE]" || data.is_empty() {
        return;
    }

    if let Ok(json) = serde_json::from_str::<Value>(&data) {
        let kind = event_type
            .or_else(|| json.get("type").and_then(Value::as_str).map(ToOwned::to_owned))
            .unwrap_or_default();

        if kind == "content_block_start" {
            if let Some(block) = json.get("content_block") {
                if block.get("type").and_then(Value::as_str) == Some("tool_use")
                    && block.get("name").and_then(Value::as_str) == Some("Agent")
                {
                    *parent_tool_use_id = block
                        .get("id")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned);

                    // 从 input 中提取 subagent_type
                    if let Some(input) = block.get("input") {
                        *agent_type = input
                            .get("subagent_type")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned);
                    }
                }
            }
        }
    }
}

fn int_field(value: &Value, key: &str) -> i32 {
    value
        .get(key)
        .and_then(Value::as_i64)
        .unwrap_or_default() as i32
}