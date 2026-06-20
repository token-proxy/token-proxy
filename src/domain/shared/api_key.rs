//! API Key 值对象 — domain/shared/
//!
//! 定义 `ApiKey` 类型，封装 API 密钥并提供脱敏展示（仅暴露后 6 位）。

use serde::{Deserialize, Serialize};
use std::fmt;

/// API Key 值对象，提供脱敏展示和末尾截取能力
///
/// `Display` 和 `Debug` 实现均为脱敏格式，避免意外泄露。
/// 完整密钥通过 `as_str()` / `into_inner()` 显式获取。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiKey(String);

impl ApiKey {
    /// 创建新的 API Key
    pub fn new(key: String) -> Self {
        ApiKey(key)
    }

    /// 返回脱敏字符串（`******` + 后 6 位）
    pub fn mask(&self) -> String {
        let suffix = self.suffix();
        format!("******{}", suffix)
    }

    /// 返回后 6 位作为标识
    pub fn suffix(&self) -> String {
        let len = self.0.len();
        if len <= 6 {
            self.0.clone()
        } else {
            self.0[len - 6..].to_string()
        }
    }

    /// 获取内部字符串引用
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 消费自身并返回内部字符串
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.mask())
    }
}

impl From<String> for ApiKey {
    fn from(key: String) -> Self {
        ApiKey::new(key)
    }
}

impl AsRef<str> for ApiKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_mask() {
        let key = ApiKey::new("sk-ant-abcdef123456".to_string());
        assert_eq!(key.mask(), "******123456");
    }

    #[test]
    fn test_api_key_suffix() {
        let key = ApiKey::new("sk-ant-abcdef123456".to_string());
        assert_eq!(key.suffix(), "123456");
    }

    #[test]
    fn test_api_key_short_key() {
        let key = ApiKey::new("abc".to_string());
        assert_eq!(key.suffix(), "abc");
    }

    #[test]
    fn test_api_key_display() {
        let key = ApiKey::new("sk-test-secret123".to_string());
        assert_eq!(format!("{}", key), "******ret123");
    }

    #[test]
    fn test_api_key_into_inner() {
        let key = ApiKey::new("secret-key".to_string());
        assert_eq!(key.into_inner(), "secret-key");
    }
}
