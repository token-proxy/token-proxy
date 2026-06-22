//! 短码值对象 — domain/access_point/
//!
//! 定义 `ShortCode` 类型，封装接入点的 URL 短码（4-16 位字母数字），
//! 提供校验、随机生成和字符串转换能力。

use crate::shared::error::AppError;
use rand::RngExt;
use sea_orm::DeriveValueType;
use serde::{Deserialize, Serialize};
use std::fmt;

const MIN_LENGTH: usize = 4;
const MAX_LENGTH: usize = 16;
const GENERATED_LENGTH: usize = 16;
const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

/// 接入点短码值对象（4-16 位字母数字，含下划线和连字符）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, DeriveValueType)]
pub struct ShortCode(String);

impl ShortCode {
    /// 创建一个新的 ShortCode，校验后存储
    pub fn new(s: &str) -> Result<Self, AppError> {
        let trimmed = s.trim();
        if trimmed.len() < MIN_LENGTH || trimmed.len() > MAX_LENGTH {
            return Err(AppError::Validation(format!(
                "短码长度必须在 {} 到 {} 个字符之间, 当前长度: {}",
                MIN_LENGTH,
                MAX_LENGTH,
                trimmed.len()
            )));
        }
        if !trimmed
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(AppError::Validation(
                "短码只能包含字母、数字、下划线和连字符".to_string(),
            ));
        }
        Ok(ShortCode(trimmed.to_string()))
    }

    /// 随机生成 16 位字母数字短码
    pub fn generate() -> Self {
        let mut rng = rand::rng();
        let code: String = (0..GENERATED_LENGTH)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        ShortCode(code)
    }

    /// 获取内部字符串引用
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ShortCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ShortCode {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_code_new_valid() {
        let code = ShortCode::new("abc123").unwrap();
        assert_eq!(code.as_str(), "abc123");
    }

    #[test]
    fn test_short_code_new_too_short() {
        let result = ShortCode::new("ab");
        assert!(result.is_err());
    }

    #[test]
    fn test_short_code_new_too_long() {
        let result = ShortCode::new("abcdefghijklmnopqrstuvwxyz123");
        assert!(result.is_err());
    }

    #[test]
    fn test_short_code_new_invalid_chars() {
        let result = ShortCode::new("hello world");
        assert!(result.is_err());
    }

    #[test]
    fn test_short_code_new_with_special_chars() {
        let code = ShortCode::new("my-code_123").unwrap();
        assert_eq!(code.as_str(), "my-code_123");
    }

    #[test]
    fn test_short_code_generate() {
        let code = ShortCode::generate();
        assert_eq!(code.as_str().len(), 16);
        assert!(code.as_str().chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_short_code_display() {
        let code = ShortCode::new("test1234").unwrap();
        assert_eq!(format!("{}", code), "test1234");
    }

    #[test]
    fn test_short_code_as_ref() {
        let code = ShortCode::new("test5678").unwrap();
        let s: &str = code.as_ref();
        assert_eq!(s, "test5678");
    }

    #[test]
    fn test_short_code_trim() {
        let code = ShortCode::new("  abcd  ").unwrap();
        assert_eq!(code.as_str(), "abcd");
    }
}
