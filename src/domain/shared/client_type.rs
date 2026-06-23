//! 客户端类型识别模块。
//!
//! 所属层级：领域共享层。提供请求客户端的归一化分类，与 API 协议类型正交。
//!
//! 识别策略：仅基于 User-Agent 关键词匹配。已知品牌 → 对应枚举值；
//! UA 存在但非已知 → Other；无 UA → Unknown。

use axum::http::HeaderMap;
use sea_orm::prelude::StringLen;
use sea_orm::DeriveActiveEnum;
use sea_orm::EnumIter;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// 归一化的客户端类型枚举。
///
/// 与 [`super::AccessPointType`] 正交——同一个 OpenAI 接入点可被多种客户端访问。
/// 识别逻辑见 [`ClientType::from_request`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum ClientType {
    /// Claude Code CLI（UA 含 claude-cli）
    #[sea_orm(string_value = "claude_code")]
    ClaudeCode,
    /// Codex CLI（UA 含 codex_cli_rs 或 codex-tui）
    #[sea_orm(string_value = "codex")]
    Codex,
    /// 其他可识别但非已知品牌的客户端
    #[sea_orm(string_value = "other")]
    Other,
    /// 完全不可识别的客户端
    #[sea_orm(string_value = "unknown")]
    Unknown,
}

impl fmt::Display for ClientType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientType::ClaudeCode => write!(f, "claude_code"),
            ClientType::Codex => write!(f, "codex"),
            ClientType::Other => write!(f, "other"),
            ClientType::Unknown => write!(f, "unknown"),
        }
    }
}

impl FromStr for ClientType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude_code" => Ok(ClientType::ClaudeCode),
            "codex" => Ok(ClientType::Codex),
            "other" => Ok(ClientType::Other),
            "unknown" => Ok(ClientType::Unknown),
            _ => Err(format!("不支持的客户端类型: {}", s)),
        }
    }
}

impl ClientType {
    /// 从请求头中的 User-Agent 字符串识别客户端类型。
    ///
    /// 识别策略（仅基于 UA）：
    /// 1. UA 关键词匹配已知品牌 → 对应枚举值
    /// 2. UA 存在但非已知 → Other
    /// 3. 无 UA → Unknown
    pub fn from_request(headers: &HeaderMap) -> Self {
        let ua = headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if ua.is_empty() {
            return Self::Unknown;
        }

        if ua.contains("claude-cli") {
            return Self::ClaudeCode;
        }
        if ua.contains("codex_cli_rs/") || ua.contains("codex-tui/") {
            return Self::Codex;
        }

        Self::Other
    }

    /// 根据客户端类型提取会话标识。
    ///
    /// 不同客户端使用不同的 header 传递会话/线程 ID：
    /// - Claude Code → `x-claude-code-session-id`
    /// - Codex → `thread-id`（仅连字符形式）
    /// - 其他 → None
    pub fn extract_session_id(&self, headers: &HeaderMap) -> Option<String> {
        match self {
            Self::ClaudeCode => headers
                .get("x-claude-code-session-id")?
                .to_str()
                .ok()
                .map(String::from),
            Self::Codex => headers.get("thread-id")?.to_str().ok().map(String::from),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderName, HeaderValue};

    fn headers_with(key: &str, value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            HeaderName::from_str(key).unwrap(),
            HeaderValue::from_str(value).unwrap(),
        );
        h
    }

    // ─── from_request UA 关键词匹配 ────────────────────────────────────

    #[test]
    fn test_recognize_claude_code_by_ua() {
        let h = headers_with("user-agent", "claude-cli/1.2.3 (linux)");
        assert_eq!(ClientType::from_request(&h), ClientType::ClaudeCode);
    }

    #[test]
    fn test_recognize_codex_by_ua() {
        let h = headers_with(
            "user-agent",
            "codex_cli_rs/0.142.0 (Windows 10.0.22631; x86_64) unknown",
        );
        assert_eq!(ClientType::from_request(&h), ClientType::Codex);
    }

    #[test]
    fn test_recognize_codex_tui_by_ua() {
        let h = headers_with(
            "user-agent",
            "codex-tui/0.142.0 (Windows 10.0.22631; x86_64) unknown (codex-tui; 0.142.0)",
        );
        assert_eq!(ClientType::from_request(&h), ClientType::Codex);
    }

    // ─── from_request 降级 ─────────────────────────────────────────────

    #[test]
    fn test_unknown_when_no_ua_header() {
        let h = HeaderMap::new();
        assert_eq!(ClientType::from_request(&h), ClientType::Unknown);
    }

    #[test]
    fn test_other_for_unrecognized_ua() {
        let h = headers_with("user-agent", "python-requests/2.31.0");
        assert_eq!(ClientType::from_request(&h), ClientType::Other);
    }

    // ─── extract_session_id ─────────────────────────────────────────────

    #[test]
    fn test_extract_session_id_claude_code() {
        let h = headers_with("x-claude-code-session-id", "sess-123");
        let ct = ClientType::ClaudeCode;
        assert_eq!(ct.extract_session_id(&h), Some("sess-123".to_string()));
    }

    #[test]
    fn test_extract_session_id_codex() {
        let h = headers_with("thread-id", "thread-456");
        let ct = ClientType::Codex;
        assert_eq!(ct.extract_session_id(&h), Some("thread-456".to_string()));
    }

    #[test]
    fn test_extract_session_id_other_returns_none() {
        let h = headers_with("thread-id", "x");
        assert_eq!(ClientType::Other.extract_session_id(&h), None);
        assert_eq!(ClientType::Unknown.extract_session_id(&h), None);
    }

    // ─── Display 和 FromStr ────────────────────────────────────────────

    #[test]
    fn test_display_and_from_str() {
        assert_eq!(ClientType::ClaudeCode.to_string(), "claude_code");
        assert_eq!("codex".parse::<ClientType>().unwrap(), ClientType::Codex);
        assert_eq!("other".parse::<ClientType>().unwrap(), ClientType::Other);
        assert_eq!(
            "unknown".parse::<ClientType>().unwrap(),
            ClientType::Unknown
        );
    }
}
