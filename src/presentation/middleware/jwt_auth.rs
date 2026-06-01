use axum::{
    extract::{FromRequestParts, Request, State},
    http::request::Parts,
    middleware::Next,
    response::Response,
};

use crate::application::AppState;
use crate::shared::error::AppError;

/// JWT 认证中间件
///
/// 对所有路由进行 JWT Bearer token 认证。
/// 公开路由和代理路由已通过路由分组排除，因此此中间件只做一件事：验证 JWT。
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("缺少认证令牌".to_string()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("无效的认证头格式".to_string()))?;

    let claims = state.auth_service.validate_access_token(token).await?;

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
