use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;

use crate::application::provider::dto::{
    AccountResponse, CreateAccountRequest, SetAccountStatusRequest, UpdateAccountRequest,
};
use crate::application::AppState;
use crate::domain::shared::Status;
use crate::presentation::middleware::jwt_auth::CurrentUser;
use crate::shared::error::AppError;

/// 构建账号管理路由
///
/// 账号是嵌套在服务商之下的子资源:
/// - `GET    /api/providers/{provider_id}/accounts`          → list_accounts
/// - `POST   /api/providers/{provider_id}/accounts`          → create_account
/// - `GET    /api/providers/{provider_id}/accounts/{id}`     → get_account
/// - `PUT    /api/providers/{provider_id}/accounts/{id}`     → update_account
/// - `DELETE /api/providers/{provider_id}/accounts/{id}`     → delete_account
///
/// 独立的账号操作（不依赖 provider 路径）:
/// - `PUT    /api/accounts/{id}/status`                      → set_account_status
/// - `PUT    /api/accounts/{id}/recover`                     → recover_account
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
        .route("/api/accounts/{id}/status", put(set_account_status))
        .route("/api/accounts/{id}/recover", put(recover_account))
}

/// GET /api/providers/{provider_id}/accounts
///
/// 返回指定服务商下的所有账号
async fn list_accounts(
    State(state): State<AppState>,
    Path(provider_id): Path<Uuid>,
) -> Result<Json<Vec<AccountResponse>>, AppError> {
    let accounts = state.account_service.list_by_provider(provider_id).await?;
    Ok(Json(accounts))
}

/// POST /api/providers/{provider_id}/accounts
///
/// 为指定服务商创建新账号
async fn create_account(
    State(state): State<AppState>,
    Path(provider_id): Path<Uuid>,
    CurrentUser(user_id): CurrentUser,
    Json(req): Json<CreateAccountRequest>,
) -> Result<Json<AccountResponse>, AppError> {
    let account = state
        .account_service
        .create(Some(user_id), provider_id, req)
        .await?;
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
    CurrentUser(user_id): CurrentUser,
    Json(req): Json<UpdateAccountRequest>,
) -> Result<Json<AccountResponse>, AppError> {
    let account = state.account_service.update(Some(user_id), id, req).await?;
    Ok(Json(account))
}

/// DELETE /api/providers/{provider_id}/accounts/{id}
///
/// 删除指定账号
async fn delete_account(
    State(state): State<AppState>,
    Path((_provider_id, id)): Path<(Uuid, Uuid)>,
    CurrentUser(user_id): CurrentUser,
) -> Result<Json<serde_json::Value>, AppError> {
    state.account_service.delete(Some(user_id), id).await?;
    Ok(Json(serde_json::json!({"message": "账号已删除"})))
}

/// PUT /api/accounts/{id}/status
///
/// 启用或禁用账号
async fn set_account_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    CurrentUser(user_id): CurrentUser,
    Json(req): Json<SetAccountStatusRequest>,
) -> Result<Json<AccountResponse>, AppError> {
    let status: Status = req
        .status
        .parse()
        .map_err(|e: AppError| AppError::Validation(e.to_string()))?;
    let account = state
        .account_service
        .set_status(Some(user_id), id, status)
        .await?;
    Ok(Json(account))
}

/// PUT /api/accounts/{id}/recover
///
/// 恢复被自动禁用的账号（清除 disabled_reason 和 available_at，重置为启用）
async fn recover_account(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    CurrentUser(user_id): CurrentUser,
) -> Result<Json<AccountResponse>, AppError> {
    let account = state.account_service.recover(Some(user_id), id).await?;
    Ok(Json(account))
}
