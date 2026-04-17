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
    tracing::debug!("Listing local TShock versions");
    let versions = state
        .version_manager
        .list_local()
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    tracing::debug!(count = versions.len(), "Listed local versions");
    Ok(Json(versions))
}

pub async fn available_versions(
    State(state): State<AppState>,
    _auth: Auth,
) -> Result<Json<Vec<crate::services::version_manager::VersionInfo>>, AppError> {
    tracing::info!("Fetching available TShock versions from GitHub");
    let versions = state
        .version_manager
        .fetch_available()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch available versions from GitHub");
            AppError::InternalServerError(e.to_string())
        })?;

    tracing::info!(count = versions.len(), "Fetched available versions");
    Ok(Json(versions))
}

pub async fn download_version(
    State(state): State<AppState>,
    auth: Auth,
    Json(req): Json<DownloadVersionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(user = %auth.username, version = %req.tag_name, url = %req.download_url, "Downloading TShock version");

    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can download versions".to_string(),
        ));
    }

    state
        .version_manager
        .download_version(&req.tag_name, &req.download_url)
        .await
        .map_err(|e| {
            tracing::error!(version = %req.tag_name, error = %e, "Failed to download version");
            AppError::InternalServerError(e.to_string())
        })?;

    tracing::info!(version = %req.tag_name, "Version downloaded successfully");

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
    tracing::info!(user = %auth.username, version = %version, "Deleting TShock version");

    if !auth.is_admin() {
        return Err(AppError::Forbidden(
            "Only administrators can delete versions".to_string(),
        ));
    }

    state
        .version_manager
        .delete_version(&version)
        .map_err(|e| {
            tracing::error!(version = %version, error = %e, "Failed to delete version");
            AppError::InternalServerError(e.to_string())
        })?;

    tracing::info!(version = %version, "Version deleted successfully");

    Ok(Json(json!({
        "success": true,
        "message": "Version deleted successfully"
    })))
}
