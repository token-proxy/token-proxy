//! 模型名称值对象 — domain/shared/
//!
//! 提供模型名称的规范化逻辑，解决不同客户端对同一模型使用
//! 不同写法（大小写、分隔符差异）导致统计分裂的问题。
//!
//! `normalize_model_name` 是纯函数，不依赖数据库查询。

/// 规范化模型名称：trim → lowercase → 统一分隔符为 `-`
///
/// 规则：
/// 1. 去除首尾空白
/// 2. 转换为全小写
/// 3. 将 `_` 和空格统一替换为 `-`
///
/// # 示例
///
/// ```
/// use token_proxy::domain::shared::model_name::normalize_model_name;
/// assert_eq!(normalize_model_name("DeepSeek-V4-Pro"), "deepseek-v4-pro");
/// assert_eq!(normalize_model_name("GPT_4"), "gpt-4");
/// assert_eq!(normalize_model_name("Claude Sonnet 4"), "claude-sonnet-4");
/// assert_eq!(normalize_model_name("  claude-opus-4  "), "claude-opus-4");
/// ```
pub fn normalize_model_name(raw: &str) -> String {
    raw.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c == '_' || c == ' ' { '-' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_whitespace() {
        assert_eq!(normalize_model_name("  gpt-4  "), "gpt-4");
        assert_eq!(normalize_model_name("\tgpt-4\n"), "gpt-4");
    }

    #[test]
    fn test_lowercase() {
        assert_eq!(normalize_model_name("GPT-4"), "gpt-4");
        assert_eq!(normalize_model_name("DeepSeek-V4-Pro"), "deepseek-v4-pro");
        assert_eq!(
            normalize_model_name("Claude-Sonnet-4-20250514"),
            "claude-sonnet-4-20250514"
        );
    }

    #[test]
    fn test_unify_separators() {
        assert_eq!(normalize_model_name("gpt_4"), "gpt-4");
        assert_eq!(normalize_model_name("claude opus 4"), "claude-opus-4");
        assert_eq!(normalize_model_name("gpt-4"), "gpt-4");
        assert_eq!(normalize_model_name("deepseek_v4_pro"), "deepseek-v4-pro");
    }

    #[test]
    fn test_mixed_cases() {
        assert_eq!(normalize_model_name("DeepSeek_V4_Pro"), "deepseek-v4-pro");
        assert_eq!(normalize_model_name("  GPT_4_Turbo  "), "gpt-4-turbo");
        assert_eq!(normalize_model_name("Claude Sonnet 4"), "claude-sonnet-4");
    }

    #[test]
    fn test_already_normalized() {
        assert_eq!(normalize_model_name("gpt-4"), "gpt-4");
        assert_eq!(
            normalize_model_name("claude-sonnet-4-20250514"),
            "claude-sonnet-4-20250514"
        );
    }

    #[test]
    fn test_empty_and_whitespace() {
        assert_eq!(normalize_model_name(""), "");
        assert_eq!(normalize_model_name("   "), "");
    }
}
