use async_trait::async_trait;
use crate::shared::error::AppError;

/// 解析后的 content block
///
/// 对应 Messages API 响应中的 content block 结构，支持 text、thinking、
/// redacted_thinking、tool_use、tool_result、server_tool_use 等类型。
pub struct ParsedContentBlock {
    /// block 类型: text, thinking, redacted_thinking, tool_use, tool_result, server_tool_use
    pub block_type: String,
    /// 文本内容（text/tool_result 类型时存在）
    pub content: Option<String>,
    /// thinking 内容（thinking 类型时存在）
    pub thinking_content: Option<String>,
    /// redacted_thinking 的签名（redacted_thinking 类型时存在）
    pub signature: Option<String>,
    /// tool_use 的 ID（tool_use 类型时存在）
    pub tool_use_id: Option<String>,
    /// tool_use 的名称（tool_use 类型时存在）
    pub tool_name: Option<String>,
}

/// 客户端信息
///
/// 从请求头中提取的客户端标识信息，如 Claude Code、Claude Desktop 等。
pub struct ClientInfo {
    /// 客户端名称，如 "Claude Code"
    pub client_name: Option<String>,
    /// 客户端版本号
    pub client_version: Option<String>,
    /// 客户端发布渠道
    pub client_channel: Option<String>,
    /// 客户端运行平台
    pub client_platform: Option<String>,
}

/// 解析结果
///
/// 对请求体和响应体进行解析后得到的结构化结果，
/// 包含消息摘要、content blocks、thinking 和 tool_use 信息。
pub struct ParsedLogContent {
    /// 解析器版本号，随解析逻辑变更递增
    pub parser_version: String,
    /// 消息预览（短文本，用于列表展示）
    pub message_preview: Option<String>,
    /// 消息完整内容（用于详情展示）
    pub message_full: Option<String>,
    /// 请求类型: "messages" 或 "tool_enabled_messages"
    pub request_kind: String,
    /// 主要使用的工具名称（tool_enabled_messages 类型时存在）
    pub primary_tool_name: Option<String>,
    /// 响应预览（短文本）
    pub response_preview: Option<String>,
    /// 响应中 assistant 的纯文本内容
    pub response_assistant_text: Option<String>,
    /// 解析后的 content blocks 列表
    pub content_blocks: Vec<ParsedContentBlock>,
    /// thinking 内容聚合
    pub thinking_content: Option<String>,
    /// 是否包含 thinking block
    pub has_thinking: bool,
    /// 是否包含 tool_use block
    pub has_tool_use: bool,
}

/// 日志解析器 trait
///
/// 每种 API 类型（如 Anthropic）对应一个解析器实现，
/// 负责将原始请求体和响应体解析为结构化的 `ParsedLogContent`。
#[async_trait]
pub trait LogParser: Send + Sync {
    /// 返回此解析器支持的 API 类型标识
    ///
    /// 应与 `AccessPointType` 的字符串表示一致，如 "anthropic"。
    fn api_type(&self) -> &'static str;

    /// 解析器版本号，随解析逻辑变更递增
    fn version(&self) -> &'static str;

    /// 解析请求体和响应体，返回结构化结果
    ///
    /// # 参数
    /// - `request_body`: 请求体 JSON 值
    /// - `response_body`: 响应体原始字符串（SSE 事件流或 JSON）
    ///
    /// # 返回
    /// - `Ok(ParsedLogContent)`: 解析后的结构化结果
    /// - `Err(AppError)`: 解析失败
    async fn parse(
        &self,
        request_body: &serde_json::Value,
        response_body: &str,
    ) -> Result<ParsedLogContent, AppError>;
}