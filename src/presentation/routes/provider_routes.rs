use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;

use crate::application::provider::dto::{
    CreateProviderRequest, ProviderResponse, UpdateProviderRequest,
};
use crate::application::AppState;
use crate::shared::error::AppError;

/// 构建提供商管理路由
///
/// - `GET    /api/providers`              → list_providers
/// - `POST   /api/providers`              → create_provider
/// - `GET    /api/providers/{id}`          → get_provider
/// - `PUT    /api/providers/{id}`          → update_provider
/// - `DELETE /api/providers/{id}`          → delete_provider
/// - `POST   /api/providers/{id}/models:discover` → discover_models
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/providers", get(list_providers))
        .route("/api/providers", post(create_provider))
        .route("/api/providers/{id}", get(get_provider))
        .route("/api/providers/{id}", put(update_provider))
        .route("/api/providers/{id}", delete(delete_provider))
        .route("/api/providers/{id}/models:discover", post(discover_models))
}

/// GET /api/providers
///
/// 返回所有提供商列表
async fn list_providers(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProviderResponse>>, AppError> {
    let providers = state.provider_service.list_all().await?;
    Ok(Json(providers))
}

/// POST /api/providers
///
/// 创建新的提供商
async fn create_provider(
    State(state): State<AppState>,
    Json(req): Json<CreateProviderRequest>,
) -> Result<Json<ProviderResponse>, AppError> {
    let provider = state.provider_service.create(req, None).await?;
    Ok(Json(provider))
}

/// GET /api/providers/{id}
///
/// 获取指定提供商详情
async fn get_provider(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProviderResponse>, AppError> {
    let provider = state.provider_service.get_by_id(id).await?;
    Ok(Json(provider))
}

/// PUT /api/providers/{id}
///
/// 更新指定提供商
async fn update_provider(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProviderRequest>,
) -> Result<Json<ProviderResponse>, AppError> {
    let provider = state.provider_service.update(id, req, None).await?;
    Ok(Json(provider))
}

/// DELETE /api/providers/{id}
///
/// 删除指定提供商
async fn delete_provider(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.provider_service.delete(id, None).await?;
    Ok(Json(serde_json::json!({"message": "提供商已删除"})))
}

/// POST /api/providers/{id}/models:discover
///
/// 从上游自动发现模型列表（自定义方法）
async fn discover_models(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let models = state.provider_service.discover_models(id).await?;
    Ok(Json(serde_json::json!({ "models": models })))
}
