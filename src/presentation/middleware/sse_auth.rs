//! SSE 认证中间件（展示层）
//!
//! 因浏览器 `EventSource` API 不支持自定义 HTTP header，
//! JWT 通过 URL query 参数 `?token=<jwt>` 传递。
//! 本中间件从 query 参数提取 token 并验证，认证通过后将 `Claims` 注入 request extensions。

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::application::AppState;
use crate::shared::error::AppError;

/// SSE JWT 认证中间件
///
/// 从 URL query 参数 `token` 中提取 JWT 并验证。
/// 验证通过后将 Claims 存入 request extensions，供 `CurrentUser` extractor 提取。
pub async fn sse_auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 从 URL query 参数提取 token（EventSource 不支持 Authorization header）
    let token = req
        .uri()
        .query()
        .and_then(|q| {
            q.split('&')
                .find_map(|pair| pair.strip_prefix("token=").map(|v| v.to_string()))
        })
        .ok_or_else(|| AppError::Unauthorized("缺少认证令牌".to_string()))?;

    let claims = state.auth_service.validate_access_token(&token).await?;

    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
