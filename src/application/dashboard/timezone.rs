//! 时区白名单校验。
//!
//! 用于 Dashboard 热力图端点的 `tz` query 参数校验。
//! 由于 PostgreSQL 的 `AT TIME ZONE` 不接受参数占位符（必须字面量拼接），
//! 通过 `chrono_tz::Tz::from_str` 白名单校验确保不会发生 SQL 注入。

use std::str::FromStr;

use chrono_tz::Tz;

use crate::shared::error::AppError;

/// 校验时区字符串。
///
/// 通过 `chrono_tz::Tz::from_str` 校验是否为合法 IANA 时区名。
/// 校验通过后返回标准化后的字符串，可直接拼入 SQL。
///
/// # Errors
/// - `AppError::Validation` 当 `tz` 不是合法 IANA 时区名时
pub fn validate_timezone(tz: &str) -> Result<String, AppError> {
    Tz::from_str(tz)
        .map(|t| t.name().to_string())
        .map_err(|_| AppError::Validation(format!("非法时区: {tz}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_iana_timezone() {
        assert_eq!(validate_timezone("Asia/Shanghai").unwrap(), "Asia/Shanghai");
        assert_eq!(validate_timezone("UTC").unwrap(), "UTC");
        assert_eq!(
            validate_timezone("America/New_York").unwrap(),
            "America/New_York"
        );
    }

    #[test]
    fn rejects_invalid_timezone() {
        assert!(validate_timezone("Asia/MadeUpCity").is_err());
        assert!(validate_timezone("'; DROP TABLE--").is_err());
        assert!(validate_timezone("").is_err());
    }
}
