use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;

use crate::application::dto::access_point_dto::{
    AccessPointResponse, CreateAccessPointRequest, UpdateAccessPointRequest,
};
use crate::application::AppState;
use crate::presentation::middleware::jwt_auth::CurrentUser;
use crate::shared::error::AppError;

/// 构建接入点管理路由
///
/// - `GET    /api/access-points`       → list_access_points
/// - `POST   /api/access-points`       → create_access_point（需 CurrentUser）
/// - `GET    /api/access-points/{id}`   → get_access_point
/// - `PUT    /api/access-points/{id}`   → update_access_point
/// - `DELETE /api/access-points/{id}`   → delete_access_point
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/access-points", get(list_access_points))
        .route("/api/access-points", post(create_access_point))
        .route("/api/access-points/{id}", get(get_access_point))
        .route("/api/access-points/{id}", put(update_access_point))
        .route("/api/access-points/{id}", delete(delete_access_point))
}

/// GET /api/access-points
async fn list_access_points(
    State(state): State<AppState>,
) -> Result<Json<Vec<AccessPointResponse>>, AppError> {
    let access_points = state.access_point_service.list_all().await?;
    Ok(Json(access_points))
}

/// POST /api/access-points
async fn create_access_point(
    State(state): State<AppState>,
    CurrentUser(created_by): CurrentUser,
    Json(req): Json<CreateAccessPointRequest>,
) -> Result<Json<AccessPointResponse>, AppError> {
    let access_point = state.access_point_service.create(req, created_by).await?;
    Ok(Json(access_point))
}

/// GET /api/access-points/{id}
async fn get_access_point(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AccessPointResponse>, AppError> {
    let access_point = state.access_point_service.get_by_id(id).await?;
    Ok(Json(access_point))
}

/// PUT /api/access-points/{id}
async fn update_access_point(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAccessPointRequest>,
) -> Result<Json<AccessPointResponse>, AppError> {
    let access_point = state.access_point_service.update(id, req).await?;
    Ok(Json(access_point))
}

/// DELETE /api/access-points/{id}
async fn delete_access_point(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.access_point_service.delete(id).await?;
    Ok(Json(serde_json::json!({"message": "接入点已删除"})))
}
