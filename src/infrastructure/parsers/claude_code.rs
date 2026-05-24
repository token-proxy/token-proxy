use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeContext {
    pub client_session_id: Option<String>,
    pub client_app: Option<String>,
    pub client_user_agent: Option<String>,
    pub conversation_source: String,
    pub agent_id: Option<String>,
}

pub fn parse_headers(headers: &Value) -> ClaudeCodeContext {
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

fn header_value(headers: &Value, key: &str) -> Option<String> {
    headers
        .as_object()
        .and_then(|obj| {
            obj.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .and_then(|(_, v)| v.as_str().map(ToOwned::to_owned))
        })
        .filter(|v| !v.is_empty())
}
