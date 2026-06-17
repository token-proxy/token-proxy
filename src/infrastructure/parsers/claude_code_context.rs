use axum::http::HeaderMap;

#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeContext {
    pub client_session_id: Option<String>,
    pub client_app: Option<String>,
    pub client_user_agent: Option<String>,
    pub conversation_source: String,
    pub agent_id: Option<String>,
}

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
        client_app: header_value(headers, "x-app"),
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
