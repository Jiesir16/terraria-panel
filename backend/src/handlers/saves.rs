use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{auth::Auth, error::AppError, handlers::AppState};

fn is_allowed_save_name(name: &str) -> bool {
    name.ends_with(".wld") || name.ends_with(".wld.bak") || name.ends_with(".bak")
}

fn save_source_type(name: &str, source_server_id: Option<&String>) -> String {
    if source_server_id.is_none() {
        return "manual_upload".to_string();
    }

    if name.ends_with(".zip") {
        "server_archive".to_string()
    } else {
        "server_backup".to_string()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveInfo {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub file_size: u64,
    pub source_server_id: Option<String>,
    pub source_server_name: Option<String>,
    pub source_type: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveListQuery {
    pub server_id: Option<String>,
    #[serde(default)]
    pub include_other_servers: bool,
}

pub async fn list_saves(
    State(state): State<AppState>,
    Query(query): Query<SaveListQuery>,
    _auth: Auth,
) -> Result<Json<Vec<SaveInfo>>, AppError> {
    tracing::debug!("Listing saves");

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    let sql = if query.server_id.is_some() && !query.include_other_servers {
        r#"
        SELECT sv.id, sv.name, sv.file_path, sv.file_size, sv.source_server_id, s.name, sv.created_at
        FROM saves sv
        LEFT JOIN servers s ON s.id = sv.source_server_id
        WHERE sv.source_server_id IS NULL OR sv.source_server_id = ?1
        ORDER BY sv.created_at DESC
        "#
    } else {
        r#"
        SELECT sv.id, sv.name, sv.file_path, sv.file_size, sv.source_server_id, s.name, sv.created_at
        FROM saves sv
        LEFT JOIN servers s ON s.id = sv.source_server_id
        ORDER BY sv.created_at DESC
        "#
    };

    let mut stmt = db
        .prepare(sql)
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let map_row = |row: &rusqlite::Row<'_>| {
        let source_server_id: Option<String> = row.get(4)?;
        let name: String = row.get(1)?;
        Ok(SaveInfo {
            id: row.get(0)?,
            source_type: save_source_type(&name, source_server_id.as_ref()),
            name,
            file_path: row.get(2)?,
            file_size: row.get::<_, i64>(3)? as u64,
            source_server_name: row.get(5)?,
            source_server_id,
            created_at: row.get(6)?,
        })
    };

    let saves = if let Some(server_id) = query.server_id.as_deref() {
        if query.include_other_servers {
            stmt.query_map([], map_row)
        } else {
            stmt.query_map(params![server_id], map_row)
        }
    } else {
        stmt.query_map([], map_row)
    }
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

        if !is_allowed_save_name(&filename) {
            tracing::warn!(filename = %filename, "Save upload rejected: invalid file extension");
            return Err(AppError::BadRequest(
                "Only .wld, .wld.bak and .bak files are allowed".to_string(),
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

        let saved = state
            .save_manager
            .upload_save(&filename, &data)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;

        let created_at = Utc::now().to_rfc3339();
        let save = SaveInfo {
            id: saved.id,
            name: saved.name,
            file_path: saved.file_path,
            file_size: saved.file_size,
            source_server_id: None,
            source_server_name: None,
            source_type: "manual_upload".to_string(),
            created_at: created_at.clone(),
        };

        tracing::info!(
            filename = %save.name,
            size = save.file_size,
            save_id = %save.id,
            file_path = %save.file_path,
            "Writing save file"
        );

        // Record in database
        let db = state.db.lock().map_err(|_| {
            AppError::InternalServerError("Failed to acquire database lock".to_string())
        })?;

        db.execute(
            "INSERT INTO saves (id, name, file_path, file_size, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                save.id,
                save.name,
                save.file_path,
                save.file_size as i64,
                save.created_at
            ],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        tracing::info!(user = %auth.username, save_id = %save.id, "Save uploaded successfully");
        drop(db);
        crate::db::log_operation(
            &state.db,
            &auth.user_id,
            "上传存档",
            Some(&save.name),
            Some(&format!("大小: {} bytes", save.file_size)),
        );

        return Ok(Json(json!({
            "success": true,
            "message": "Save uploaded successfully",
            "save": save
        })));
    }

    Err(AppError::BadRequest("No save file uploaded".to_string()))
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

    let (save_name, file_path): (String, String) = db
        .query_row(
            "SELECT name, file_path FROM saves WHERE id = ?1",
            params![save_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| AppError::NotFound("Save not found".to_string()))?;

    if !is_allowed_save_name(&save_name) {
        return Err(AppError::BadRequest(
            "归档包只能下载，不能直接导入为世界存档。请解压后选择其中的 .wld 文件导入。".to_string(),
        ));
    }

    tracing::debug!(
        save_id = %save_id,
        save_name = %save_name,
        file_path = %file_path,
        server_id = %server_id,
        "Copying save file to server"
    );

    let imported_world_name = state
        .save_manager
        .import_save(&file_path, &server_id, &save_name)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let now = Utc::now().to_rfc3339();
    db.execute(
        "UPDATE servers SET world_name = ?1, updated_at = ?2 WHERE id = ?3",
        params![imported_world_name, now, server_id],
    )
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    drop(db);

    tracing::info!(
        save_id = %save_id,
        server_id = %server_id,
        world_name = %save_name,
        "Save imported successfully"
    );
    crate::db::log_operation(
        &state.db,
        &auth.user_id,
        "导入存档",
        Some(&server_id),
        Some(&format!("存档: {} -> {}", save_name, imported_world_name)),
    );

    Ok(Json(json!({
        "success": true,
        "message": "Save imported successfully",
        "world_name": save_name
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

    let (save_name, file_path): (String, String) = db
        .query_row(
            "SELECT name, file_path FROM saves WHERE id = ?1",
            params![save_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| AppError::NotFound("Save not found".to_string()))?;

    drop(db);

    tracing::debug!(save_id = %save_id, file_path = %file_path, "Reading save file from disk");

    let file = std::fs::read(&file_path).map_err(|e| AppError::FileError(e.to_string()))?;

    tracing::info!(save_id = %save_id, size = file.len(), "Save file downloaded");

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    let content_disposition = format!("attachment; filename=\"{}\"", save_name);
    let content_disposition = HeaderValue::from_str(&content_disposition)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    headers.insert(header::CONTENT_DISPOSITION, content_disposition);

    Ok((StatusCode::OK, headers, file))
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
        std::fs::remove_file(&file_path).map_err(|e| AppError::FileError(e.to_string()))?;
    }

    tracing::info!(save_id = %save_id, "Save deleted successfully");
    crate::db::log_operation(
        &state.db,
        &auth.user_id,
        "删除存档",
        Some(&save_id),
        Some(&file_path),
    );

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

        let backup = state
            .save_manager
            .backup_server(&server_id, &world_name)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;

        let db = state.db.lock().map_err(|_| {
            AppError::InternalServerError("Failed to acquire database lock".to_string())
        })?;

        db.execute(
            "INSERT INTO saves (id, name, file_path, file_size, source_server_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                backup.id,
                backup.name,
                backup.file_path,
                backup.file_size as i64,
                backup.source_server_id,
                backup.created_at
            ],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        drop(db);

        crate::services::auto_backup::sync_backup_to_oss(
            &state.config.backup.oss,
            &backup.file_path,
            &backup.name,
        );
        if state.config.backup.local_retention_days > 0 {
            crate::services::auto_backup::prune_backups_older_than(
                &state.db,
                &server_id,
                state.config.backup.local_retention_days,
            );
        }

        tracing::info!(server_id = %server_id, world_name = %world_name, "Server backed up successfully");
        crate::db::log_operation(
            &state.db,
            &auth.user_id,
            "手动备份存档",
            Some(&server_id),
            Some(&world_name),
        );

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
