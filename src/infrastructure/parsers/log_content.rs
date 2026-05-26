use serde_json::Value;

/// 从 SSE 响应中解析 token 用量
///
/// 扫描 SSE 事件流寻找 message_delta 事件，提取 usage JSON。
/// token 计数是客观数据提取，不涉及内容解释。
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

fn int_field(value: &Value, key: &str) -> i32 {
    value
        .get(key)
        .and_then(Value::as_i64)
        .unwrap_or_default() as i32
}
