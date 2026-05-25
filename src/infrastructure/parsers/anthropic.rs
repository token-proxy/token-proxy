use async_trait::async_trait;
use serde_json::Value;

use crate::domain::services::{
    LogParser, ParsedContentBlock, ParsedLogContent,
};
use crate::shared::error::AppError;

/// Anthropic Messages API 日志解析器
///
/// 解析 Anthropic Messages API 的请求体和 SSE 响应体，
/// 提取用户消息、工具定义、响应内容块（text/thinking/tool_use）、
/// thinking 内容和 token 用量等信息。
pub struct AnthropicLogParser;

impl AnthropicLogParser {
    pub fn new() -> Self {
        Self
    }

    /// 从请求体中提取最后一条 role=user 的消息内容
    fn extract_last_user_message(body: &Value) -> Option<String> {
        body.get("messages")
            .and_then(Value::as_array)
            .and_then(|messages| {
                messages
                    .iter()
                    .rev()
                    .find(|message| message.get("role").and_then(Value::as_str) == Some("user"))
            })
            .and_then(|message| {
                let content = message.get("content")?;
                Self::extract_content_text(content)
            })
            .map(Self::clean_xml_tags)
            .filter(|text| !text.trim().is_empty())
    }

    /// 从 content 字段中提取文本内容
    ///
    /// content 可能是字符串或 ContentBlockParam 数组，
    /// 数组时提取所有 text 和 tool_result 类型的文本块。
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
                        item.get("content").and_then(|c| Self::extract_content_text(c))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
    }

    /// 清理 XML 标签，替换系统提醒标签为中文说明
    fn clean_xml_tags(text: String) -> String {
        text.replace("<session>", "")
            .replace("</session>", "")
            .replace("<system-reminder>", "系统提醒：")
            .replace("</system-reminder>", "")
            .trim()
            .to_string()
    }

    /// 截取文本预览（最多 200 字符，合并空白）
    fn preview(text: &str) -> String {
        const MAX_CHARS: usize = 200;
        let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if compact.chars().count() <= MAX_CHARS {
            compact
        } else {
            format!("{}...", compact.chars().take(MAX_CHARS).collect::<String>())
        }
    }

    /// 从非 SSE 响应的纯文本中提取预览
    fn preview_plain_response(response_body: &str) -> Option<String> {
        if response_body.trim().is_empty() {
            None
        } else {
            Some(Self::preview(response_body))
        }
    }

    /// 解析 SSE 事件流
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
                Self::push_sse_event(&mut events, event_type.take(), &mut data_lines);
            }
        }

        if !data_lines.is_empty() {
            Self::push_sse_event(&mut events, event_type, &mut data_lines);
        }

        events
    }

    /// 将 SSE 数据行推入事件列表
    fn push_sse_event(
        events: &mut Vec<SseEvent>,
        event_type: Option<String>,
        data_lines: &mut Vec<String>,
    ) {
        let data = data_lines.join("\n");
        data_lines.clear();

        if data == "[DONE]" || data.is_empty() {
            return;
        }

        if let Ok(json) = serde_json::from_str::<Value>(&data) {
            let kind = event_type
                .or_else(|| json.get("type").and_then(Value::as_str).map(ToOwned::to_owned));
            events.push(SseEvent {
                kind: kind.unwrap_or_else(|| "unknown".to_string()),
                index: json.get("index").and_then(Value::as_u64).map(|v| v as usize),
                data: json,
            });
        }
    }

    /// 追加文本到指定索引的 block
    fn append_to_block(blocks: &mut [(usize, String)], index: usize, text: &str) {
        if let Some((_, value)) = blocks.iter_mut().find(|(block_index, _)| *block_index == index) {
            value.push_str(text);
        }
    }

    }

#[async_trait]
impl LogParser for AnthropicLogParser {
    fn api_type(&self) -> &'static str {
        "anthropic"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    async fn parse(
        &self,
        request_body: &serde_json::Value,
        response_body: &str,
    ) -> Result<ParsedLogContent, AppError> {
        // 提取用户消息
        let message_full = Self::extract_last_user_message(request_body);
        let message_preview = message_full.as_deref().map(Self::preview);

        // 提取工具定义
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
            "messages".to_string()
        } else {
            "tool_enabled_messages".to_string()
        };
        let primary_tool_name = tools.first().cloned();

        // 解析 SSE 事件
        let sse_events = Self::parse_sse(response_body);

        // 从 SSE 事件中提取 content blocks
        let mut text_blocks: Vec<(usize, String)> = Vec::new();
        let mut thinking_blocks: Vec<(usize, String)> = Vec::new();
        let mut tool_blocks: Vec<ToolBlock> = Vec::new();

        for event in &sse_events {
            match event.kind.as_str() {
                "content_block_start" => {
                    if let Some(block) = event.data.get("content_block") {
                        match block.get("type").and_then(Value::as_str) {
                            Some("text") => {
                                text_blocks.push((event.index.unwrap_or(0), String::new()));
                            }
                            Some("thinking") => {
                                thinking_blocks.push((event.index.unwrap_or(0), String::new()));
                            }
                            Some("tool_use") => {
                                tool_blocks.push(ToolBlock {
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
                                });
                            }
                            _ => {}
                        }
                    }
                }
                "content_block_delta" => {
                    if let Some(delta) = event.data.get("delta") {
                        if let Some(text) = delta.get("text").and_then(Value::as_str) {
                            Self::append_to_block(&mut text_blocks, event.index.unwrap_or(0), text);
                        }
                        if let Some(thinking) = delta.get("thinking").and_then(Value::as_str) {
                            Self::append_to_block(&mut thinking_blocks, event.index.unwrap_or(0), thinking);
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
                _ => {}
            }
        }

        // 构建 content_blocks
        let mut content_blocks: Vec<ParsedContentBlock> = Vec::new();
        let mut response_assistant_text_parts: Vec<String> = Vec::new();
        let mut thinking_content_parts: Vec<String> = Vec::new();
        let mut has_thinking = false;
        let mut has_tool_use = false;

        // 按 index 排序合并所有 block：text + thinking + tool_use
        let max_index = text_blocks
            .iter()
            .chain(thinking_blocks.iter())
            .map(|(idx, _)| *idx)
            .chain(tool_blocks.iter().map(|t| t.index))
            .max()
            .unwrap_or(0);

        for idx in 0..=max_index {
            // 处理 thinking block
            if let Some((_, thinking_text)) = thinking_blocks.iter().find(|(i, _)| *i == idx) {
                if !thinking_text.is_empty() {
                    has_thinking = true;
                    thinking_content_parts.push(thinking_text.clone());
                    content_blocks.push(ParsedContentBlock {
                        block_type: "thinking".to_string(),
                        content: None,
                        thinking_content: Some(thinking_text.clone()),
                        signature: None,
                        tool_use_id: None,
                        tool_name: None,
                    });
                }
            }

            // 处理 tool_use block
            if let Some(tool_block) = tool_blocks.iter().find(|t| t.index == idx) {
                has_tool_use = true;
                content_blocks.push(ParsedContentBlock {
                    block_type: "tool_use".to_string(),
                    content: None,
                    thinking_content: None,
                    signature: None,
                    tool_use_id: Some(tool_block.id.clone()),
                    tool_name: Some(tool_block.name.clone()),
                });
                // tool_use 的 description 也计入 assistant text（不强制）
                if let Some(desc) = tool_block.name.strip_prefix("Agent").or(Some(&tool_block.name)) {
                    if !desc.is_empty() {
                        // 不作为文本内容添加，只标记 tool_use
                    }
                }
            }

            // 处理 text block（tool_use 之后处理，保持顺序）
            if let Some((_, text)) = text_blocks.iter().find(|(i, _)| *i == idx) {
                if !text.is_empty() {
                    response_assistant_text_parts.push(text.clone());
                    content_blocks.push(ParsedContentBlock {
                        block_type: "text".to_string(),
                        content: Some(text.clone()),
                        thinking_content: None,
                        signature: None,
                        tool_use_id: None,
                        tool_name: None,
                    });
                }
            }
        }

        // 响应预览
        let response_preview = text_blocks
            .iter()
            .find_map(|(_, value)| (!value.is_empty()).then(|| Self::preview(value)))
            .or_else(|| Self::preview_plain_response(response_body));

        let response_assistant_text = if response_assistant_text_parts.is_empty() {
            None
        } else {
            Some(response_assistant_text_parts.join("\n"))
        };

        let thinking_content = if thinking_content_parts.is_empty() {
            None
        } else {
            Some(thinking_content_parts.join("\n"))
        };

        Ok(ParsedLogContent {
            parser_version: self.version().to_string(),
            message_preview,
            message_full,
            request_kind,
            primary_tool_name,
            response_preview,
            response_assistant_text,
            content_blocks,
            thinking_content,
            has_thinking,
            has_tool_use,
        })
    }
}

// ─── 内部辅助类型 ───

/// SSE 事件结构
#[derive(Debug, Clone)]
struct SseEvent {
    kind: String,
    index: Option<usize>,
    data: Value,
}

/// Tool use block 结构
#[derive(Debug, Clone)]
struct ToolBlock {
    index: usize,
    id: String,
    name: String,
    input_json: String,
}