use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    auth::Auth,
    error::AppError,
    handlers::AppState,
};

#[derive(Debug, Deserialize)]
pub struct DownloadVersionRequest {
    pub tag_name: String,
    pub download_url: String,
}

pub async fn list_versions(
    State(state): State<AppState>,
    _auth: Auth,
) -> Result<Json<Vec<crate::services::version_manager::LocalVersion>>, AppError> {
    let versions = state
        .version_manager
        .list_local()
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(Json(versions))
}

pub async fn available_versions(
    State(state): State<AppState>,
    _auth: Auth,
) -> Result<Json<Vec<crate::services::version_manager::VersionInfo>>, AppError> {
    let versions = state
        .version_manager
        .fetch_available()
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(Json(versions))
}

pub async fn download_version(
    State(state): State<AppState>,
    auth: Auth,
    Json(req): Json<DownloadVersionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can download versions".to_string(),
        ));
    }

    state
        .version_manager
        .download_version(&req.tag_name, &req.download_url)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(Json(json!({
        "success": true,
        "message": "Version downloaded successfully",
        "version": req.tag_name
    })))
}

pub async fn delete_version(
    State(state): State<AppState>,
    auth: Auth,
    Path(version): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden(
            "Only administrators can delete versions".to_string(),
        ));
    }

    state
        .version_manager
        .delete_version(&version)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(Json(json!({
        "success": true,
        "message": "Version deleted successfully"
    })))
}
