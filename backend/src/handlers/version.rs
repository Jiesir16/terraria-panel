use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

use crate::{auth::Auth, error::AppError, handlers::AppState};

#[derive(Debug, Deserialize)]
pub struct DownloadVersionRequest {
    pub tag_name: String,
    pub download_url: String,
}

#[derive(Debug, Deserialize)]
pub struct ProxyRequest {
    pub mirror: String,
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
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<crate::services::version_manager::AvailableVersionsResponse>, AppError> {
    let page: usize = params
        .get("page")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .max(1);

    let per_page: usize = params
        .get("per_page")
        .and_then(|s| s.parse().ok())
        .unwrap_or(10)
        .min(50)
        .max(1);

    tracing::info!(
        page = page,
        per_page = per_page,
        "Fetching available TShock versions from GitHub"
    );

    let result = state
        .version_manager
        .fetch_available(page, per_page)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch available versions from GitHub");
            AppError::InternalServerError(e.to_string())
        })?;

    tracing::info!(
        total = result.total,
        page = result.page,
        "Fetched available versions"
    );
    Ok(Json(result))
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
    crate::db::log_operation(
        &state.db,
        &auth.user_id,
        "下载版本",
        Some(&req.tag_name),
        Some(&req.download_url),
    );

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
    crate::db::log_operation(&state.db, &auth.user_id, "删除版本", Some(&version), None);

    Ok(Json(json!({
        "success": true,
        "message": "Version deleted successfully"
    })))
}

/// Get current GitHub proxy/mirror setting
pub async fn get_proxy(
    State(state): State<AppState>,
    _auth: Auth,
) -> Result<Json<serde_json::Value>, AppError> {
    let mirror = state.version_manager.get_github_mirror().await;
    Ok(Json(json!({
        "mirror": mirror
    })))
}

/// Set GitHub proxy/mirror URL
pub async fn set_proxy(
    State(state): State<AppState>,
    auth: Auth,
    Json(req): Json<ProxyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can change proxy settings".to_string(),
        ));
    }

    let mirror = req.mirror.trim().to_string();
    tracing::info!(user = %auth.username, mirror = %mirror, "Updating GitHub mirror/proxy");

    state
        .version_manager
        .set_github_mirror(mirror.clone())
        .await;
    crate::db::log_operation(
        &state.db,
        &auth.user_id,
        "更新代理设置",
        None,
        Some(&mirror),
    );

    Ok(Json(json!({
        "success": true,
        "message": "Proxy settings updated",
        "mirror": mirror
    })))
}
