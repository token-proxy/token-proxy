use serde_json::Value;

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

pub fn parse(request_body: &Value, response_body: &str) -> ParsedLogContent {
    let message_full = extract_last_user_message(request_body);
    let message_preview = message_full.as_deref().map(preview);
    let tools = request_body
        .get("tools")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(Value::as_str).map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let request_kind = if tools.is_empty() {
        Some("messages".to_string())
    } else {
        Some("tool_enabled_messages".to_string())
    };
    let primary_tool_name = tools.first().cloned();

    let sse_events = parse_sse(response_body);
    let mut events = Vec::new();
    if let Some(text) = message_full.clone() {
        events.push(ParsedConversationEvent {
            role: "user".to_string(),
            event_type: "user_message".to_string(),
            tool_use_id: None,
            tool_name: None,
            title: None,
            content: Some(text),
            thinking_content: None,
            display_payload: None,
            confidence: 100,
        });
    }

    let mut text_blocks: Vec<(usize, String)> = Vec::new();
    let mut thinking_blocks: Vec<(usize, String)> = Vec::new();
    let mut tool_blocks: Vec<ToolBlock> = Vec::new();
    let mut usage = None;

    for event in &sse_events {
        match event.kind.as_str() {
            "content_block_start" => {
                if let Some(block) = event.data.get("content_block") {
                    match block.get("type").and_then(Value::as_str) {
                        Some("text") => text_blocks.push((event.index.unwrap_or(0), String::new())),
                        Some("thinking") => thinking_blocks.push((event.index.unwrap_or(0), String::new())),
                        Some("tool_use") => tool_blocks.push(ToolBlock {
                            index: event.index.unwrap_or(0),
                            id: block
                                .get("id")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            name: block
                                .get("name")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            input_json: String::new(),
                        }),
                        _ => {}
                    }
                }
            }
            "content_block_delta" => {
                if let Some(delta) = event.data.get("delta") {
                    if let Some(text) = delta.get("text").and_then(Value::as_str) {
                        append_to_block(&mut text_blocks, event.index.unwrap_or(0), text);
                    }
                    if let Some(thinking) = delta.get("thinking").and_then(Value::as_str) {
                        append_to_block(&mut thinking_blocks, event.index.unwrap_or(0), thinking);
                    }
                    if let Some(partial) = delta.get("partial_json").and_then(Value::as_str) {
                        if let Some(block) = tool_blocks
                            .iter_mut()
                            .find(|block| block.index == event.index.unwrap_or(0))
                        {
                            block.input_json.push_str(partial);
                        }
                    }
                }
            }
            "message_delta" => {
                if let Some(raw_usage) = event.data.get("usage") {
                    usage = parse_usage(raw_usage.clone());
                }
            }
            _ => {}
        }
    }

    for (_, thinking) in thinking_blocks.iter().filter(|(_, value)| !value.is_empty()) {
        events.push(ParsedConversationEvent {
            role: "assistant".to_string(),
            event_type: "assistant_thinking".to_string(),
            tool_use_id: None,
            tool_name: None,
            title: Some("思考过程".to_string()),
            content: None,
            thinking_content: Some(thinking.clone()),
            display_payload: None,
            confidence: 100,
        });
    }

    for block in &tool_blocks {
        let input = serde_json::from_str::<Value>(&block.input_json).unwrap_or(Value::Null);
        events.push(ParsedConversationEvent {
            role: "assistant".to_string(),
            event_type: if block.name == "Agent" {
                "agent_call".to_string()
            } else {
                "tool_use".to_string()
            },
            tool_use_id: Some(block.id.clone()),
            tool_name: Some(block.name.clone()),
            title: tool_title(&block.name, &input),
            content: None,
            thinking_content: None,
            display_payload: Some(input),
            confidence: 100,
        });
    }

    for (_, text) in text_blocks.iter().filter(|(_, value)| !value.is_empty()) {
        events.push(ParsedConversationEvent {
            role: "assistant".to_string(),
            event_type: "assistant_message".to_string(),
            tool_use_id: None,
            tool_name: None,
            title: None,
            content: Some(text.clone()),
            thinking_content: None,
            display_payload: None,
            confidence: 100,
        });
    }

    let response_preview = text_blocks
        .iter()
        .find_map(|(_, value)| (!value.is_empty()).then(|| preview(value)))
        .or_else(|| preview_plain_response(response_body));

    let agent_call = tool_blocks.iter().find(|block| block.name == "Agent");
    let agent_payload = agent_call.and_then(|block| serde_json::from_str::<Value>(&block.input_json).ok());

    ParsedLogContent {
        message_preview,
        message_full,
        response_preview,
        request_kind,
        primary_tool_name,
        has_thinking: events
            .iter()
            .any(|event| event.event_type == "assistant_thinking"),
        has_tool_use: !tool_blocks.is_empty(),
        events,
        usage,
        agent_type: agent_payload
            .as_ref()
            .and_then(|value| value.get("subagent_type"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        parent_agent_tool_use_id: agent_call.map(|block| block.id.clone()),
    }
}

fn extract_last_user_message(body: &Value) -> Option<String> {
    body.get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| {
            messages
                .iter()
                .rev()
                .find(|message| message.get("role").and_then(Value::as_str) == Some("user"))
        })
        .and_then(|message| extract_content_text(message.get("content")?))
        .map(clean_xml_tags)
        .filter(|text| !text.trim().is_empty())
}

fn extract_content_text(content: &Value) -> Option<String> {
    if let Some(text) = content.as_str() {
        return Some(text.to_string());
    }

    content.as_array().map(|items| {
        items
            .iter()
            .filter_map(|item| {
                if item.get("type").and_then(Value::as_str) == Some("text") {
                    item.get("text").and_then(Value::as_str).map(ToOwned::to_owned)
                } else if item.get("type").and_then(Value::as_str) == Some("tool_result") {
                    item.get("content").and_then(extract_content_text)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn clean_xml_tags(text: String) -> String {
    text.replace("<session>", "")
        .replace("</session>", "")
        .replace("<system-reminder>", "系统提醒：")
        .replace("</system-reminder>", "")
        .trim()
        .to_string()
}

fn preview(text: &str) -> String {
    const MAX_CHARS: usize = 200;
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= MAX_CHARS {
        compact
    } else {
        format!("{}...", compact.chars().take(MAX_CHARS).collect::<String>())
    }
}

fn preview_plain_response(response_body: &str) -> Option<String> {
    if response_body.trim().is_empty() {
        None
    } else {
        Some(preview(response_body))
    }
}

fn append_to_block(blocks: &mut [(usize, String)], index: usize, text: &str) {
    if let Some((_, value)) = blocks.iter_mut().find(|(block_index, _)| *block_index == index) {
        value.push_str(text);
    }
}

fn tool_title(name: &str, input: &Value) -> Option<String> {
    if name == "Agent" {
        input
            .get("description")
            .and_then(Value::as_str)
            .map(|description| format!("启动子代理：{}", description))
    } else {
        Some(format!("工具调用：{}", name))
    }
}

fn parse_usage(raw_usage: Value) -> Option<ParsedTokenUsage> {
    let input = int_field(&raw_usage, "input_tokens");
    let output = int_field(&raw_usage, "output_tokens");
    let cache_creation = int_field(&raw_usage, "cache_creation_input_tokens");
    let cache_read = int_field(&raw_usage, "cache_read_input_tokens");
    let thinking = int_field(&raw_usage, "thinking_tokens");

    Some(ParsedTokenUsage {
        input_tokens: input,
        output_tokens: output,
        cache_creation_input_tokens: cache_creation,
        cache_read_input_tokens: cache_read,
        thinking_tokens: thinking,
        total_tokens: input + output + cache_creation + cache_read + thinking,
        raw_usage,
    })
}

fn int_field(value: &Value, key: &str) -> i32 {
    value
        .get(key)
        .and_then(Value::as_i64)
        .unwrap_or_default() as i32
}

fn parse_sse(response_body: &str) -> Vec<SseEvent> {
    let mut events = Vec::new();
    let mut event_type = None;
    let mut data_lines = Vec::new();

    for line in response_body.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            event_type = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("data:") {
            data_lines.push(value.trim().to_string());
        } else if line.trim().is_empty() && !data_lines.is_empty() {
            push_sse_event(&mut events, event_type.take(), &mut data_lines);
        }
    }

    if !data_lines.is_empty() {
        push_sse_event(&mut events, event_type, &mut data_lines);
    }

    events
}

fn push_sse_event(events: &mut Vec<SseEvent>, event_type: Option<String>, data_lines: &mut Vec<String>) {
    let data = data_lines.join("\n");
    data_lines.clear();

    if data == "[DONE]" || data.is_empty() {
        return;
    }

    if let Ok(json) = serde_json::from_str::<Value>(&data) {
        let kind = event_type.or_else(|| json.get("type").and_then(Value::as_str).map(ToOwned::to_owned));
        events.push(SseEvent {
            kind: kind.unwrap_or_else(|| "unknown".to_string()),
            index: json.get("index").and_then(Value::as_u64).map(|value| value as usize),
            data: json,
        });
    }
}

#[derive(Debug, Clone)]
struct SseEvent {
    kind: String,
    index: Option<usize>,
    data: Value,
}

#[derive(Debug, Clone)]
struct ToolBlock {
    index: usize,
    id: String,
    name: String,
    input_json: String,
}
