/// HTTP User-Agent 解析器
///
/// 从 User-Agent 字符串中提取客户端信息, 支持格式:
/// `claude-cli/2.1.146 (external, cli)`
///
/// 解析规则:
/// - 斜杠 `/` 前为 client_name
/// - 斜杠 `/` 后到第一个空白字符为 client_version
/// - 括号 `()` 内以逗号分隔: 前为 client_channel, 后为 client_platform

/// 客户端信息
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ClientInfo {
    /// 客户端名称, 如 `claude-cli`
    pub client_name: Option<String>,
    /// 客户端版本, 如 `2.1.146`
    pub client_version: Option<String>,
    /// 客户端发布渠道, 如 `external`
    pub client_channel: Option<String>,
    /// 客户端平台, 如 `cli`
    pub client_platform: Option<String>,
}

/// 从 User-Agent 字符串中解析客户端信息
///
/// # 参数
/// * `user_agent` - HTTP User-Agent 字符串
///
/// # 返回值
/// 返回 `ClientInfo`, 任何字段缺失时对应字段为 `None`
pub fn parse_user_agent(user_agent: &str) -> ClientInfo {
    if user_agent.is_empty() {
        return ClientInfo::default();
    }

    let (client_name, rest_after_name) = match user_agent.split_once('/') {
        Some((name, rest)) => (Some(name.to_string()), rest),
        None => {
            // 没有斜杠, 只有名称无版本和括号信息
            return ClientInfo {
                client_name: Some(user_agent.to_string()),
                client_version: None,
                client_channel: None,
                client_platform: None,
            };
        }
    };

    let (client_version, rest_after_version) = match rest_after_name.split_once(' ') {
        Some((version, rest)) => (Some(version.to_string()), rest),
        None => {
            // 版本后没有空白字符, 只有名称和版本
            return ClientInfo {
                client_name,
                client_version: Some(rest_after_name.to_string()),
                client_channel: None,
                client_platform: None,
            };
        }
    };

    // 解析括号内的渠道和平台信息: (channel, platform)
    let (client_channel, client_platform) = parse_parenthesized(rest_after_version);

    ClientInfo {
        client_name,
        client_version,
        client_channel,
        client_platform,
    }
}

/// 解析括号内的逗号分隔信息
///
/// 格式: `(channel, platform)`
/// 返回 `(client_channel, client_platform)`, 无法解析时均为 `None`
fn parse_parenthesized(input: &str) -> (Option<String>, Option<String>) {
    let trimmed = input.trim();

    // 检查是否以 '(' 开头且以 ')' 结尾
    let inner = if trimmed.starts_with('(') && trimmed.ends_with(')') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        return (None, None);
    };

    match inner.split_once(',') {
        Some((channel, platform)) => {
            let channel = channel.trim();
            let platform = platform.trim();
            (
                if channel.is_empty() { None } else { Some(channel.to_string()) },
                if platform.is_empty() { None } else { Some(platform.to_string()) },
            )
        }
        None => {
            // 有括号但无逗号, 整体作为 channel
            let channel = inner.trim();
            if channel.is_empty() {
                (None, None)
            } else {
                (Some(channel.to_string()), None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 标准格式: claude-cli/2.1.146 (external, cli)
    #[test]
    fn test_standard_format() {
        let result = parse_user_agent("claude-cli/2.1.146 (external, cli)");
        assert_eq!(
            result,
            ClientInfo {
                client_name: Some("claude-cli".to_string()),
                client_version: Some("2.1.146".to_string()),
                client_channel: Some("external".to_string()),
                client_platform: Some("cli".to_string()),
            }
        );
    }

    /// 缺少括号信息: claude-cli/2.1.146
    #[test]
    fn test_missing_parenthesized() {
        let result = parse_user_agent("claude-cli/2.1.146");
        assert_eq!(
            result,
            ClientInfo {
                client_name: Some("claude-cli".to_string()),
                client_version: Some("2.1.146".to_string()),
                client_channel: None,
                client_platform: None,
            }
        );
    }

    /// 空字符串
    #[test]
    fn test_empty_string() {
        let result = parse_user_agent("");
        assert_eq!(result, ClientInfo::default());
    }

    /// 只有名称无版本
    #[test]
    fn test_name_only() {
        let result = parse_user_agent("claude-cli");
        assert_eq!(
            result,
            ClientInfo {
                client_name: Some("claude-cli".to_string()),
                client_version: None,
                client_channel: None,
                client_platform: None,
            }
        );
    }

    /// 有名称和版本, 括号内无逗号
    #[test]
    fn test_parenthesized_without_comma() {
        let result = parse_user_agent("claude-cli/2.1.146 (external)");
        assert_eq!(
            result,
            ClientInfo {
                client_name: Some("claude-cli".to_string()),
                client_version: Some("2.1.146".to_string()),
                client_channel: Some("external".to_string()),
                client_platform: None,
            }
        );
    }

    /// 括号内有额外空白
    #[test]
    fn test_extra_whitespace_in_parentheses() {
        let result = parse_user_agent("claude-cli/2.1.146 (  external ,  cli  )");
        assert_eq!(
            result,
            ClientInfo {
                client_name: Some("claude-cli".to_string()),
                client_version: Some("2.1.146".to_string()),
                client_channel: Some("external".to_string()),
                client_platform: Some("cli".to_string()),
            }
        );
    }

    /// 空括号
    #[test]
    fn test_empty_parentheses() {
        let result = parse_user_agent("claude-cli/2.1.146 ()");
        assert_eq!(
            result,
            ClientInfo {
                client_name: Some("claude-cli".to_string()),
                client_version: Some("2.1.146".to_string()),
                client_channel: None,
                client_platform: None,
            }
        );
    }
}