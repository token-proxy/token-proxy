use axum::{
    extract::{FromRequestParts, Request, State},
    http::request::Parts,
    middleware::Next,
    response::Response,
};

use crate::application::AppState;
use crate::shared::error::AppError;

/// API key 认证中间件
///
/// 对 `/ap/*` 代理转发路径进行用户 API key 认证。
/// 从 `Authorization: Bearer <token>` 提取 token，
/// 通过 SHA-256 哈希在数据库中查找并验证状态。
/// 认证通过后将 `ApiKeyUser` 注入 request extensions。
pub async fn middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("缺少 Authorization 请求头".to_string()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Authorization 格式应为 Bearer <token>".to_string()))?;

    let user_id = state.user_api_key_service.validate_api_key(token).await?;

    req.extensions_mut().insert(ApiKeyUser(user_id));

    Ok(next.run(req).await)
}

/// 从 request extensions 中提取通过 API key 认证的用户 ID
#[derive(Debug, Clone, Copy)]
pub struct ApiKeyUser(pub uuid::Uuid);

impl<S> FromRequestParts<S> for ApiKeyUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<ApiKeyUser>()
            .copied()
            .ok_or_else(|| AppError::Unauthorized("未认证".to_string()))
    }
}
