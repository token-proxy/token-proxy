use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::error::AppError;

/// JWT 声明
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// 主题（用户 ID 的字符串形式）
    pub sub: String,
    /// 用户 UUID
    pub user_id: Uuid,
    /// 用户名
    pub username: String,
    /// 过期时间戳（秒）
    pub exp: usize,
    /// 签发时间戳（秒）
    pub iat: usize,
}

/// JWT 令牌服务
pub struct JwtService {
    secret: String,
    access_expiry_secs: i64,
    refresh_expiry_secs: i64,
}

impl JwtService {
    /// 创建 JWT 服务实例
    ///
    /// * `secret` - 签名密钥
    /// * `access_expiry_secs` - 访问令牌有效期（秒）
    /// * `refresh_expiry_secs` - 刷新令牌有效期（秒）
    pub fn new(secret: String, access_expiry_secs: i64, refresh_expiry_secs: i64) -> Self {
        JwtService {
            secret,
            access_expiry_secs,
            refresh_expiry_secs,
        }
    }

    /// 获取访问令牌过期时间（秒）
    pub fn access_expiry_secs(&self) -> u64 {
        self.access_expiry_secs as u64
    }

    /// 创建访问令牌（短寿命）
    pub fn create_access_token(&self, user_id: Uuid, username: &str) -> Result<String, AppError> {
        let now = Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: user_id.to_string(),
            user_id,
            username: username.to_string(),
            exp: now + self.access_expiry_secs as usize,
            iat: now,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(format!("JWT 签名失败: {}", e)))
    }

    /// 创建刷新令牌（长寿命）
    pub fn create_refresh_token(&self, user_id: Uuid, username: &str) -> Result<String, AppError> {
        let now = Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: user_id.to_string(),
            user_id,
            username: username.to_string(),
            exp: now + self.refresh_expiry_secs as usize,
            iat: now,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(format!("JWT 签名失败: {}", e)))
    }

    /// 验证令牌并返回声明
    pub fn validate_token(&self, token: &str) -> Result<Claims, AppError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| {
            match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    AppError::Unauthorized("令牌已过期".to_string())
                }
                jsonwebtoken::errors::ErrorKind::InvalidToken
                | jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                    AppError::Unauthorized("无效的令牌".to_string())
                }
                _ => AppError::Unauthorized(format!("令牌验证失败: {}", e)),
            }
        })?;

        Ok(token_data.claims)
    }
}