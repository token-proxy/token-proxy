use axum::routing::get;
use axum::{extract::State, Json, Router};

use crate::application::system::dto::UpdateSettingsRequest;
use crate::application::AppState;
use crate::presentation::middleware::jwt_auth::CurrentUser;
use crate::shared::error::AppError;

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/settings", get(get_settings).put(update_settings))
}

async fn get_settings(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let settings = state.settings_service.get_settings().await?;
    Ok(Json(serde_json::json!(settings)))
}

async fn update_settings(
    State(state): State<AppState>,
    CurrentUser(user_id): CurrentUser,
    Json(input): Json<UpdateSettingsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let settings = state
        .settings_service
        .update_settings(input, Some(user_id))
        .await?;
    Ok(Json(serde_json::json!(settings)))
}
