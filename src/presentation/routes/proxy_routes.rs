use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::HeaderMap,
    response::Response,
    routing::post,
    Router,
};

use crate::application::AppState;
use crate::presentation::middleware::user_api_key_auth::ApiKeyUser;
use crate::shared::error::AppError;

/// 构建代理转发路由（API key 认证中间件在路由分组层应用）
///
/// - `POST /ap/{short_code}/{*remainder}` → proxy_messages
///   remainder 捕获所有后续路径段（如 v1/messages），透明透传到上游
///
/// `max_body_bytes` 为入站请求体上限。代理管道需要拿到完整 body 才能改写模型名，
/// 因此 body 走 `String` 提取器整体缓冲；多模态请求把图片/PDF 以 base64 内联进
/// JSON，体积远超 axum 默认的 2 MiB，必须在路由上显式放宽，否则请求在提取阶段
/// 就被 axum 以 413 `length limit exceeded` 拒绝，根本到不了管道。
pub fn routes(max_body_bytes: usize) -> Router<AppState> {
    Router::new()
        .route("/ap/{short_code}/{*remainder}", post(proxy_messages))
        .layer(DefaultBodyLimit::max(max_body_bytes))
}

/// POST /ap/{short_code}/{*remainder}
///
/// 核心代理转发入口。认证由 API key 中间件完成，
/// 转发编排由 ProxyPipeline 负责。
async fn proxy_messages(
    State(state): State<AppState>,
    Path((short_code, remainder)): Path<(String, String)>,
    ApiKeyUser(user_id): ApiKeyUser,
    headers: HeaderMap,
    body: String,
) -> Result<Response, AppError> {
    state
        .proxy_pipeline
        .execute(&short_code, &remainder, headers, body, user_id)
        .await
}
