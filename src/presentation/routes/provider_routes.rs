use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;

use crate::application::dto::provider_dto::{
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
/// - `POST   /api/providers/{id}/discover-models` → discover_models
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/providers", get(list_providers))
        .route("/api/providers", post(create_provider))
        .route("/api/providers/{id}", get(get_provider))
        .route("/api/providers/{id}", put(update_provider))
        .route("/api/providers/{id}", delete(delete_provider))
        .route(
            "/api/providers/{id}/discover-models",
            post(discover_models),
        )
}

/// GET /api/providers
async fn list_providers(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProviderResponse>>, AppError> {
    let providers = state.provider_service.list_all().await?;
    Ok(Json(providers))
}

/// POST /api/providers
async fn create_provider(
    State(state): State<AppState>,
    Json(req): Json<CreateProviderRequest>,
) -> Result<Json<ProviderResponse>, AppError> {
    let provider = state.provider_service.create(req).await?;
    Ok(Json(provider))
}

/// GET /api/providers/{id}
async fn get_provider(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProviderResponse>, AppError> {
    let provider = state.provider_service.get_by_id(id).await?;
    Ok(Json(provider))
}

/// PUT /api/providers/{id}
async fn update_provider(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProviderRequest>,
) -> Result<Json<ProviderResponse>, AppError> {
    let provider = state.provider_service.update(id, req).await?;
    Ok(Json(provider))
}

/// DELETE /api/providers/{id}
async fn delete_provider(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.provider_service.delete(id).await?;
    Ok(Json(serde_json::json!({"message": "提供商已删除"})))
}

/// POST /api/providers/{id}/discover-models
///
/// Phase 1: 占位接口，返回空列表
async fn discover_models(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> Result<Json<Vec<String>>, AppError> {
    // Phase 1 返回空列表，后续会实现真正的模型发现
    Ok(Json(Vec::new()))
}