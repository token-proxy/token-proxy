use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;

use crate::application::provider::account_dto::{
    AccountResponse, CreateAccountRequest, UpdateAccountRequest,
};
use crate::application::AppState;
use crate::shared::error::AppError;

/// 构建账号管理路由
///
/// 账号是嵌套在提供商之下的子资源:
/// - `GET    /api/providers/{provider_id}/accounts`          → list_accounts
/// - `POST   /api/providers/{provider_id}/accounts`          → create_account
/// - `GET    /api/providers/{provider_id}/accounts/{id}`     → get_account
/// - `PUT    /api/providers/{provider_id}/accounts/{id}`     → update_account
/// - `DELETE /api/providers/{provider_id}/accounts/{id}`     → delete_account
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/providers/{provider_id}/accounts", get(list_accounts))
        .route(
            "/api/providers/{provider_id}/accounts",
            post(create_account),
        )
        .route(
            "/api/providers/{provider_id}/accounts/{id}",
            get(get_account),
        )
        .route(
            "/api/providers/{provider_id}/accounts/{id}",
            put(update_account),
        )
        .route(
            "/api/providers/{provider_id}/accounts/{id}",
            delete(delete_account),
        )
}

/// GET /api/providers/{provider_id}/accounts
///
/// 返回指定提供商下的所有账号
async fn list_accounts(
    State(state): State<AppState>,
    Path(provider_id): Path<Uuid>,
) -> Result<Json<Vec<AccountResponse>>, AppError> {
    let accounts = state.account_service.list_by_provider(provider_id).await?;
    Ok(Json(accounts))
}

/// POST /api/providers/{provider_id}/accounts
///
/// 为指定提供商创建新账号
async fn create_account(
    State(state): State<AppState>,
    Path(provider_id): Path<Uuid>,
    Json(req): Json<CreateAccountRequest>,
) -> Result<Json<AccountResponse>, AppError> {
    let account = state.account_service.create(provider_id, req).await?;
    Ok(Json(account))
}

/// GET /api/providers/{provider_id}/accounts/{id}
///
/// 获取指定账号详情
async fn get_account(
    State(state): State<AppState>,
    Path((_provider_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<AccountResponse>, AppError> {
    let account = state.account_service.get_by_id(id).await?;
    Ok(Json(account))
}

/// PUT /api/providers/{provider_id}/accounts/{id}
///
/// 更新指定账号
async fn update_account(
    State(state): State<AppState>,
    Path((_provider_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateAccountRequest>,
) -> Result<Json<AccountResponse>, AppError> {
    let account = state.account_service.update(id, req).await?;
    Ok(Json(account))
}

/// DELETE /api/providers/{provider_id}/accounts/{id}
///
/// 删除指定账号
async fn delete_account(
    State(state): State<AppState>,
    Path((_provider_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.account_service.delete(id).await?;
    Ok(Json(serde_json::json!({"message": "账号已删除"})))
}
