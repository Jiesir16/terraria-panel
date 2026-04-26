use axum::{
    extract::{Multipart, Path, State},
    Json,
};
use serde_json::json;

use crate::{auth::Auth, error::AppError, handlers::AppState, models::ModList};

pub async fn list_mods(
    State(state): State<AppState>,
    _auth: Auth,
    Path(server_id): Path<String>,
) -> Result<Json<ModList>, AppError> {
    tracing::debug!(server_id = %server_id, "Listing mods");
    let mod_list = state
        .mod_manager
        .list_mods(&server_id)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    tracing::debug!(server_id = %server_id, count = mod_list.total, "Listed mods");
    Ok(Json(mod_list))
}

pub async fn upload_mod(
    State(state): State<AppState>,
    auth: Auth,
    Path(server_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(user = %auth.username, server_id = %server_id, "Uploading mod");

    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can upload mods".to_string(),
        ));
    }

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::FileError(e.to_string()))?
    {
        let filename = field
            .file_name()
            .ok_or_else(|| AppError::BadRequest("Missing filename".to_string()))?
            .to_string();

        // Only allow .dll files
        if !filename.ends_with(".dll") {
            tracing::warn!(server_id = %server_id, filename = %filename, "Mod upload rejected: invalid file extension");
            return Err(AppError::BadRequest(
                "Only .dll files are allowed".to_string(),
            ));
        }

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::FileError(e.to_string()))?
            .to_vec();

        // Check file size (max 50MB)
        if data.len() > 50 * 1024 * 1024 {
            tracing::warn!(server_id = %server_id, filename = %filename, size = data.len(), "Mod upload rejected: file too large");
            return Err(AppError::BadRequest(
                "File size exceeds 50MB limit".to_string(),
            ));
        }

        tracing::info!(server_id = %server_id, filename = %filename, size = data.len(), "Writing mod file");

        state
            .mod_manager
            .upload_mod(&server_id, &filename, &data)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    }

    tracing::info!(user = %auth.username, server_id = %server_id, "Mod uploaded successfully");
    crate::db::log_operation(&state.db, &auth.user_id, "上传模组", Some(&server_id), None);

    Ok(Json(json!({
        "success": true,
        "message": "Mod uploaded successfully"
    })))
}

pub async fn toggle_mod(
    State(state): State<AppState>,
    auth: Auth,
    Path((server_id, mod_name)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(user = %auth.username, server_id = %server_id, mod_name = %mod_name, "Toggling mod");

    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can toggle mods".to_string(),
        ));
    }

    state
        .mod_manager
        .toggle_mod(&server_id, &mod_name)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    tracing::info!(server_id = %server_id, mod_name = %mod_name, "Mod toggled successfully");
    crate::db::log_operation(
        &state.db,
        &auth.user_id,
        "切换模组",
        Some(&server_id),
        Some(&mod_name),
    );

    Ok(Json(json!({
        "success": true,
        "message": "Mod toggled successfully"
    })))
}

pub async fn delete_mod(
    State(state): State<AppState>,
    auth: Auth,
    Path((server_id, mod_name)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(user = %auth.username, server_id = %server_id, mod_name = %mod_name, "Deleting mod");

    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can delete mods".to_string(),
        ));
    }

    state
        .mod_manager
        .delete_mod(&server_id, &mod_name)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    tracing::info!(server_id = %server_id, mod_name = %mod_name, "Mod deleted successfully");
    crate::db::log_operation(
        &state.db,
        &auth.user_id,
        "删除模组",
        Some(&server_id),
        Some(&mod_name),
    );

    Ok(Json(json!({
        "success": true,
        "message": "Mod deleted successfully"
    })))
}
