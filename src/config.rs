use std::time::Duration;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_access_expiry: Duration,
    pub jwt_refresh_expiry: Duration,
    pub encryption_key: [u8; 32],
    pub server_port: u16,
    pub log_level: String,
    /// 分区检查间隔（秒），默认 3600（1 小时）
    pub partition_check_interval_secs: u64,
    /// 提前创建未来分区数，默认 3
    pub partition_premake_months: u32,
    /// 分区保留月数，默认 12
    pub partition_retention_months: u32,
    /// 代理转发入站请求体上限（字节），默认 64 MiB
    ///
    /// 多模态请求会把图片/PDF 以 base64 内联进 JSON body，体积远超 axum
    /// 默认的 2 MiB，超出时请求在提取 body 阶段即被 axum 以 413 拒绝。
    pub proxy_max_body_bytes: usize,
}

/// 代理请求体上限默认值：64 MiB
///
/// 需覆盖多模态请求的 base64 膨胀（Anthropic Messages API 自身请求上限为 32 MB，
/// base64 后约 43 MB），同时保留一道防内存滥用的硬上限。
pub const DEFAULT_PROXY_MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

/// 解析代理请求体上限（字节），必须是正整数
fn parse_body_limit(raw: &str) -> Result<usize, String> {
    let value = raw
        .trim()
        .parse::<usize>()
        .map_err(|_| "PROXY_MAX_BODY_BYTES 必须是有效的字节数（正整数）".to_string())?;

    if value == 0 {
        return Err("PROXY_MAX_BODY_BYTES 必须大于 0".to_string());
    }

    Ok(value)
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let database_url =
            std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL 环境变量未设置".to_string())?;

        let jwt_secret =
            std::env::var("JWT_SECRET").map_err(|_| "JWT_SECRET 环境变量未设置".to_string())?;

        let encryption_key_hex = std::env::var("ENCRYPTION_KEY")
            .map_err(|_| "ENCRYPTION_KEY 环境变量未设置".to_string())?;

        let encryption_key = hex_to_bytes(&encryption_key_hex)?;

        let server_port = std::env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|_| "SERVER_PORT 必须是有效端口号".to_string())?;

        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        let partition_check_interval_secs = std::env::var("PARTITION_CHECK_INTERVAL_SECS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse::<u64>()
            .map_err(|_| "PARTITION_CHECK_INTERVAL_SECS 必须是有效的秒数".to_string())?;

        let partition_premake_months = std::env::var("PARTITION_PREMAKE_MONTHS")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u32>()
            .map_err(|_| "PARTITION_PREMAKE_MONTHS 必须是有效数字".to_string())?;

        let partition_retention_months = std::env::var("PARTITION_RETENTION_MONTHS")
            .unwrap_or_else(|_| "12".to_string())
            .parse::<u32>()
            .map_err(|_| "PARTITION_RETENTION_MONTHS 必须是有效数字".to_string())?;

        let proxy_max_body_bytes = parse_body_limit(
            &std::env::var("PROXY_MAX_BODY_BYTES")
                .unwrap_or_else(|_| DEFAULT_PROXY_MAX_BODY_BYTES.to_string()),
        )?;

        Ok(Config {
            database_url,
            jwt_secret,
            jwt_access_expiry: Duration::from_secs(30 * 60), // 30 分钟
            jwt_refresh_expiry: Duration::from_secs(7 * 24 * 3600), // 7 天
            encryption_key,
            server_port,
            log_level,
            partition_check_interval_secs,
            partition_premake_months,
            partition_retention_months,
            proxy_max_body_bytes,
        })
    }
}

fn hex_to_bytes(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 {
        return Err(format!(
            "ENCRYPTION_KEY 必须是 64 位十六进制字符串 (32 字节), 当前长度: {}",
            hex.len()
        ));
    }

    let mut bytes = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        if i >= 32 {
            return Err("ENCRYPTION_KEY 格式错误".to_string());
        }
        let high = hex_char_to_val(chunk[0])?;
        let low = hex_char_to_val(if chunk.len() > 1 { chunk[1] } else { b'0' })?;
        bytes[i] = (high << 4) | low;
    }

    Ok(bytes)
}

fn hex_char_to_val(c: u8) -> Result<u8, String> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(format!(
            "ENCRYPTION_KEY 包含无效十六进制字符: '{}'",
            c as char
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_body_limit_accepts_positive_integer() {
        assert_eq!(parse_body_limit("1048576"), Ok(1048576));
        assert_eq!(parse_body_limit(" 67108864 "), Ok(67108864));
    }

    #[test]
    fn default_body_limit_covers_multimodal_requests() {
        // 2 MiB 是 axum 默认值，多模态请求必须显著高于它才有意义
        assert!(DEFAULT_PROXY_MAX_BODY_BYTES > 2 * 1024 * 1024);
    }

    #[test]
    fn parse_body_limit_rejects_zero_and_garbage() {
        assert!(parse_body_limit("0").is_err());
        assert!(parse_body_limit("").is_err());
        assert!(parse_body_limit("abc").is_err());
        assert!(parse_body_limit("-1").is_err());
    }
}
