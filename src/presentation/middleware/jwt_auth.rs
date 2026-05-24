use axum::{
    extract::{FromRequestParts, Request, State},
    http::request::Parts,
    middleware::Next,
    response::Response,
};

use crate::application::AppState;
use crate::shared::error::AppError;

/// 公开路径（无需认证）
const PUBLIC_PATHS: &[&str] = &["/api/auth/login", "/api/auth/refresh", "/api/health"];

/// `/ap/*` 代理转发路径也是公开的
fn is_public_path(path: &str) -> bool {
    if path.starts_with("/ap/") {
        return true;
    }
    PUBLIC_PATHS.iter().any(|p| path.starts_with(p))
}

/// JWT 认证中间件
///
/// 对所有请求进行拦截检查:
/// - 公开路径直接放行
/// - 非公开路径需要有效的 Bearer token
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 1. 检查是否为公开路径
    if is_public_path(req.uri().path()) {
        return Ok(next.run(req).await);
    }

    // 2. 提取 Authorization header
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("缺少认证令牌".to_string()))?;

    // 3. 验证 Bearer token 格式
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("无效的认证头格式".to_string()))?;

    // 4. 验证 JWT
    let claims = state.auth_service.validate_access_token(token).await?;

    // 5. 将用户信息注入 request extensions
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}

/// 从 request extensions 中提取当前用户 ID
#[derive(Debug, Clone, Copy)]
pub struct CurrentUser(pub uuid::Uuid);

impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<crate::application::dto::auth_dto::Claims>()
            .ok_or_else(|| AppError::Unauthorized("未认证".to_string()))?;
        Ok(CurrentUser(claims.user_id))
    }
}
