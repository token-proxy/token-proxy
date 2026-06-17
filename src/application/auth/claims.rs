use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT 声明（应用层 DTO）
///
/// 由 AuthService::validate_access_token 从基础设施层的 Claims 映射而来。
/// 注意：infrastructure::auth::claims::Claims 是 jsonwebtoken 的编解码类型，两者不同。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub user_id: Uuid,
    pub username: String,
    pub exp: usize,
    pub iat: usize,
}
