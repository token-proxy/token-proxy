//! Claude Code 上下文解析器（基础设施层）
//!
//! 从入站请求头中提取 Claude Code 特有的上下文信息，
//! 如客户端会话 ID、agent ID 和会话来源类型。

use axum::http::HeaderMap;

/// Claude Code 上下文信息
///
/// 用于区分主会话、子 agent 会话和未知来源。
#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeContext {
    /// 客户端会话 ID（`x-claude-code-session-id` 请求头）
    pub client_session_id: Option<String>,
    /// User-Agent 字符串
    pub client_user_agent: Option<String>,
    /// 会话来源类型：`main` / `subagent` / `unknown`
    pub conversation_source: String,
    /// Claude Code agent ID（`x-claude-code-agent-id` 请求头）
    pub agent_id: Option<String>,
}

/// 从 HTTP 请求头中解析 Claude Code 上下文
///
/// 提取 `x-claude-code-session-id`、`x-claude-code-agent-id`、
/// `x-app` 和 `user-agent` 请求头。
pub fn parse_headers(headers: &HeaderMap) -> ClaudeCodeContext {
    let client_session_id = header_value(headers, "x-claude-code-session-id");
    let agent_id = header_value(headers, "x-claude-code-agent-id");
    let conversation_source = if agent_id.is_some() {
        "subagent".to_string()
    } else if client_session_id.is_some() {
        "main".to_string()
    } else {
        "unknown".to_string()
    };

    ClaudeCodeContext {
        client_session_id,
        client_user_agent: header_value(headers, "user-agent"),
        conversation_source,
        agent_id,
    }
}

fn header_value(headers: &HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
}
