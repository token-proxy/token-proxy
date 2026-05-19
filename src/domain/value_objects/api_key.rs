use serde::{Deserialize, Serialize};
use std::fmt;

/// API Key 值对象，包装原始 API Key 字符串
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiKey(String);

impl ApiKey {
    /// 创建一个新的 ApiKey（不校验格式）
    pub fn new(key: String) -> Self {
        ApiKey(key)
    }

    /// 返回掩码后的字符串："******" + 末尾 6 位
    pub fn mask(&self) -> String {
        let suffix = self.suffix();
        format!("******{}", suffix)
    }

    /// 返回末尾 6 位
    pub fn suffix(&self) -> String {
        let len = self.0.len();
        if len <= 6 {
            self.0.clone()
        } else {
            self.0[len - 6..].to_string()
        }
    }

    /// 获取内部引用
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 获取内部字符串（消费 self）
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
        assert_eq!(key.mask(), "******abc");
    }

    #[test]
    fn test_api_key_display() {
        let key = ApiKey::new("sk-test-secret123".to_string());
        let display = format!("{}", key);
        assert_eq!(display, "******ret123");
    }

    #[test]
    fn test_api_key_into_inner() {
        let key = ApiKey::new("secret-key".to_string());
        assert_eq!(key.into_inner(), "secret-key");
    }
}