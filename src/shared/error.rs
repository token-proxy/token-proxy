use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("校验失败: {0}")]
    Validation(String),

    #[error("资源未找到: {0}")]
    NotFound(String),

    #[error("资源冲突: {0}")]
    Conflict(String),

    #[error("未认证: {0}")]
    Unauthorized(String),

    #[error("无权限: {0}")]
    Forbidden(String),

    #[error("加密错误: {0}")]
    Encryption(String),

    #[error("数据库错误: {0}")]
    Database(String),

    #[error("上游服务错误: {0}")]
    Upstream(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::Encryption(_) => (StatusCode::INTERNAL_SERVER_ERROR, "内部加密错误".to_string()),
            AppError::Database(msg) => {
                error!(%msg, "数据库错误");
                (StatusCode::INTERNAL_SERVER_ERROR, "内部数据库错误".to_string())
            }
            AppError::Upstream(msg) => {
                error!(%msg, "上游服务错误");
                (StatusCode::BAD_GATEWAY, format!("上游服务错误: {}", msg))
            }
            AppError::Internal(msg) => {
                error!(%msg, "内部错误");
                (StatusCode::INTERNAL_SERVER_ERROR, "内部服务器错误".to_string())
            }
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}