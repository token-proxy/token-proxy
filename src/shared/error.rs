//! 应用层错误类型定义（共享层）
//!
//! 定义了 9 种 `AppError` 变体，对应不同的 HTTP 状态码。
//! 实现了 `IntoResponse`，确保所有错误都以统一 JSON 格式返回。
//! `Database` / `Upstream` / `Internal` 错误会额外记录 `tracing::error!` 日志。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use sea_orm::DbErr;
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

impl From<DbErr> for AppError {
    fn from(e: DbErr) -> Self {
        AppError::Database(e.to_string())
    }
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
            AppError::Encryption(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "内部加密错误".to_string(),
            ),
            AppError::Database(msg) => {
                error!(%msg, "数据库错误");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "内部数据库错误".to_string(),
                )
            }
            AppError::Upstream(msg) => {
                error!(%msg, "上游服务错误");
                (StatusCode::BAD_GATEWAY, format!("上游服务错误: {}", msg))
            }
            AppError::Internal(msg) => {
                error!(%msg, "内部错误");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "内部服务器错误".to_string(),
                )
            }
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}
