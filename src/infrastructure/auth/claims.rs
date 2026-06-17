use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT 声明（基础设施层）
///
/// 用于 jsonwebtoken 的编码/解码。
/// application 层有一个同名的 `Claims` struct（DTO 层面），两者是不同的类型。
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
