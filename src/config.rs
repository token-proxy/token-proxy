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
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL 环境变量未设置".to_string())?;

        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET 环境变量未设置".to_string())?;

        let encryption_key_hex = std::env::var("ENCRYPTION_KEY")
            .map_err(|_| "ENCRYPTION_KEY 环境变量未设置".to_string())?;

        let encryption_key = hex_to_bytes(&encryption_key_hex)?;

        let server_port = std::env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|_| "SERVER_PORT 必须是有效端口号".to_string())?;

        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        Ok(Config {
            database_url,
            jwt_secret,
            jwt_access_expiry: Duration::from_secs(30 * 60),      // 30 分钟
            jwt_refresh_expiry: Duration::from_secs(7 * 24 * 3600), // 7 天
            encryption_key,
            server_port,
            log_level,
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
        _ => Err(format!("ENCRYPTION_KEY 包含无效十六进制字符: '{}'", c as char)),
    }
}