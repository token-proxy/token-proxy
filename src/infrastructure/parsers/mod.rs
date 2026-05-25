pub mod anthropic;
pub mod claude_code;
pub mod log_content;
pub mod user_agent;

/// 根据 API 类型创建对应的日志解析器
///
/// 当前所有 API 类型默认使用 Anthropic 解析器，
/// 未来可扩展为多解析器路由。
pub fn create_parser(api_type: &str) -> Box<dyn crate::domain::services::LogParser> {
    match api_type {
        "anthropic" => Box::new(anthropic::AnthropicLogParser::new()),
        _ => Box::new(anthropic::AnthropicLogParser::new()),
    }
}