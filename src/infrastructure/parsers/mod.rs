//! 请求/响应解析器（基础设施层）
//!
//! 包括 User-Agent 解析、Claude Code 上下文解析和 Token 用量解析。

pub mod claude_code_context;
pub mod client_info;
pub mod parsed_token_usage;

pub use claude_code_context::ClaudeCodeContext;
pub use client_info::ClientInfo;
pub use parsed_token_usage::ParsedTokenUsage;
