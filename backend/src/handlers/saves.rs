use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use chrono::Utc;
use rusqlite::params;

use crate::{
    auth::Auth,
    error::AppError,
    handlers::AppState,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveInfo {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub file_size: u64,
    pub source_server_id: Option<String>,
    pub created_at: String,
}

pub async fn list_saves(
    State(state): State<AppState>,
    _auth: Auth,
) -> Result<Json<Vec<SaveInfo>>, AppError> {
    tracing::debug!("Listing saves");

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    let mut stmt = db
        .prepare("SELECT id, name, file_path, file_size, source_server_id, created_at FROM saves")
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let saves = stmt
        .query_map([], |row| {
            Ok(SaveInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                file_path: row.get(2)?,
                file_size: row.get::<_, i64>(3)? as u64,
                source_server_id: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    tracing::debug!(count = saves.len(), "Listed saves");
    Ok(Json(saves))
}

pub async fn upload_save(
    State(state): State<AppState>,
    auth: Auth,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(user = %auth.username, "Uploading save file");

    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can upload saves".to_string(),
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

        // Only allow .wld files
        if !filename.ends_with(".wld") {
            tracing::warn!(filename = %filename, "Save upload rejected: invalid file extension");
            return Err(AppError::BadRequest(
                "Only .wld files are allowed".to_string(),
            ));
        }

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::FileError(e.to_string()))?
            .to_vec();

        // Check file size (max 500MB)
        if data.len() > 500 * 1024 * 1024 {
            tracing::warn!(filename = %filename, size = data.len(), "Save upload rejected: file too large");
            return Err(AppError::BadRequest(
                "File size exceeds 500MB limit".to_string(),
            ));
        }

        let save_id = Uuid::new_v4().to_string();
        let file_size = data.len() as u64;

        tracing::info!(filename = %filename, size = file_size, save_id = %save_id, "Writing save file");

        state
            .save_manager
            .upload_save(&filename, &data)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;

        // Record in database
        let db = state.db.lock().map_err(|_| {
            AppError::InternalServerError("Failed to acquire database lock".to_string())
        })?;

        let now = Utc::now().to_rfc3339();
        let file_path = state
            .config
            .server
            .data_dir
            .join("saves")
            .join(&filename)
            .to_string_lossy()
            .to_string();

        db.execute(
            "INSERT INTO saves (id, name, file_path, file_size, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![save_id, filename, file_path, file_size as i64, now],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    tracing::info!(user = %auth.username, "Save uploaded successfully");

    Ok(Json(json!({
        "success": true,
        "message": "Save uploaded successfully"
    })))
}

pub async fn import_save(
    State(state): State<AppState>,
    auth: Auth,
    Path((save_id, server_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(user = %auth.username, save_id = %save_id, server_id = %server_id, "Importing save to server");

    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can import saves".to_string(),
        ));
    }

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    let file_path: String = db
        .query_row(
            "SELECT file_path FROM saves WHERE id = ?1",
            params![save_id],
            |row| row.get(0),
        )
        .map_err(|_| AppError::NotFound("Save not found".to_string()))?;

    drop(db);

    tracing::debug!(save_id = %save_id, file_path = %file_path, server_id = %server_id, "Copying save file to server");

    state
        .save_manager
        .import_save(&file_path, &server_id)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    tracing::info!(save_id = %save_id, server_id = %server_id, "Save imported successfully");

    Ok(Json(json!({
        "success": true,
        "message": "Save imported successfully"
    })))
}

pub async fn download_save(
    State(state): State<AppState>,
    _auth: Auth,
    Path(save_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(save_id = %save_id, "Downloading save file");

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    let file_path: String = db
        .query_row(
            "SELECT file_path FROM saves WHERE id = ?1",
            params![save_id],
            |row| row.get(0),
        )
        .map_err(|_| AppError::NotFound("Save not found".to_string()))?;

    drop(db);

    tracing::debug!(save_id = %save_id, file_path = %file_path, "Reading save file from disk");

    let file = std::fs::read(&file_path)
        .map_err(|e| AppError::FileError(e.to_string()))?;

    let _filename = std::path::Path::new(&file_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    tracing::info!(save_id = %save_id, size = file.len(), "Save file downloaded");

    Ok((
        StatusCode::OK,
        [("content-type", "application/octet-stream")],
        file,
    ))
}

pub async fn delete_save(
    State(state): State<AppState>,
    auth: Auth,
    Path(save_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(user = %auth.username, save_id = %save_id, "Deleting save");

    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can delete saves".to_string(),
        ));
    }

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    let file_path: String = db
        .query_row(
            "SELECT file_path FROM saves WHERE id = ?1",
            params![save_id],
            |row| row.get(0),
        )
        .map_err(|_| AppError::NotFound("Save not found".to_string()))?;

    db.execute("DELETE FROM saves WHERE id = ?1", params![save_id])
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    drop(db);

    // Delete file
    if std::path::Path::new(&file_path).exists() {
        tracing::debug!(save_id = %save_id, file_path = %file_path, "Removing save file from disk");
        std::fs::remove_file(&file_path)
            .map_err(|e| AppError::FileError(e.to_string()))?;
    }

    tracing::info!(save_id = %save_id, "Save deleted successfully");

    Ok(Json(json!({
        "success": true,
        "message": "Save deleted successfully"
    })))
}

pub async fn backup_server(
    State(state): State<AppState>,
    auth: Auth,
    Path(server_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(user = %auth.username, server_id = %server_id, "Backing up server world");

    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can backup servers".to_string(),
        ));
    }

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    let world_name: Option<String> = db
        .query_row(
            "SELECT world_name FROM servers WHERE id = ?1",
            params![server_id],
            |row| row.get(0),
        )
        .map_err(|_| AppError::NotFound("Server not found".to_string()))?;

    drop(db);

    if let Some(world_name) = world_name {
        tracing::info!(server_id = %server_id, world_name = %world_name, "Creating backup of world");

        state
            .save_manager
            .backup_server(&server_id, &world_name)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;

        tracing::info!(server_id = %server_id, world_name = %world_name, "Server backed up successfully");

        Ok(Json(json!({
            "success": true,
            "message": "Server backed up successfully"
        })))
    } else {
        tracing::warn!(server_id = %server_id, "Backup failed: no world name set");
        Err(AppError::BadRequest(
            "Server has no world name set".to_string(),
        ))
    }
}
