use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;
use uuid::Uuid;
use chrono::Utc;
use rusqlite::params;

use crate::{
    auth::Auth,
    error::AppError,
    models::{CreateServerRequest, UpdateServerRequest, Server, ServerDetail, CommandRequest},
    handlers::AppState,
};

pub async fn list_servers(
    State(state): State<AppState>,
    _auth: Auth,
) -> Result<Json<Vec<Server>>, AppError> {
    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    let mut stmt = db
        .prepare("SELECT id, name, port, tshock_version, world_name, status, password, max_players, auto_start, created_by, created_at, updated_at FROM servers")
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let servers = stmt
        .query_map([], |row| {
            Ok(Server {
                id: row.get(0)?,
                name: row.get(1)?,
                port: row.get(2)?,
                tshock_version: row.get(3)?,
                world_name: row.get(4)?,
                status: row.get(5)?,
                password: row.get(6)?,
                max_players: row.get(7)?,
                auto_start: row.get(8)?,
                created_by: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(Json(servers))
}

pub async fn create_server(
    State(state): State<AppState>,
    auth: Auth,
    Json(req): Json<CreateServerRequest>,
) -> Result<Json<Server>, AppError> {
    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can create servers".to_string(),
        ));
    }

    let server_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let port = req.port.unwrap_or(7777);
    let max_players = req.max_players.unwrap_or(8);
    let auto_start = req.auto_start.unwrap_or(false);

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    db.execute(
        "INSERT INTO servers (id, name, port, tshock_version, world_name, status, password, max_players, auto_start, created_by, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            server_id,
            req.name,
            port,
            req.tshock_version,
            req.world_name,
            "stopped",
            req.password,
            max_players,
            auto_start as i32,
            auth.user_id,
            now,
            now
        ],
    )
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Create server directories
    let server_dir = state.config.server.data_dir.join("servers").join(&server_id);
    std::fs::create_dir_all(server_dir.join("world"))
        .map_err(|e| AppError::FileError(e.to_string()))?;
    std::fs::create_dir_all(server_dir.join("ServerPlugins"))
        .map_err(|e| AppError::FileError(e.to_string()))?;
    std::fs::create_dir_all(server_dir.join("logs"))
        .map_err(|e| AppError::FileError(e.to_string()))?;
    std::fs::create_dir_all(server_dir.join("tshock"))
        .map_err(|e| AppError::FileError(e.to_string()))?;

    Ok(Json(Server {
        id: server_id,
        name: req.name,
        port,
        tshock_version: req.tshock_version,
        world_name: req.world_name,
        status: "stopped".to_string(),
        password: req.password,
        max_players,
        auto_start,
        created_by: Some(auth.user_id),
        created_at: now.clone(),
        updated_at: now,
    }))
}

pub async fn get_server(
    State(state): State<AppState>,
    _auth: Auth,
    Path(id): Path<String>,
) -> Result<Json<ServerDetail>, AppError> {
    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    let server = db
        .query_row(
            "SELECT id, name, port, tshock_version, world_name, status, password, max_players, auto_start, created_by, created_at, updated_at FROM servers WHERE id = ?1",
            params![id],
            |row| {
                Ok(Server {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    port: row.get(2)?,
                    tshock_version: row.get(3)?,
                    world_name: row.get(4)?,
                    status: row.get(5)?,
                    password: row.get(6)?,
                    max_players: row.get(7)?,
                    auto_start: row.get(8)?,
                    created_by: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            },
        )
        .map_err(|_| AppError::NotFound("Server not found".to_string()))?;

    Ok(Json(ServerDetail {
        server,
        player_count: 0,
        uptime_seconds: 0,
    }))
}

pub async fn update_server(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<String>,
    Json(req): Json<UpdateServerRequest>,
) -> Result<Json<Server>, AppError> {
    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can update servers".to_string(),
        ));
    }

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    let now = Utc::now().to_rfc3339();

    // Build update query
    let mut update_count = 0;
    if let Some(name) = &req.name {
        db.execute(
            "UPDATE servers SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![name, now, id],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        update_count += 1;
    }
    if let Some(port) = &req.port {
        db.execute(
            "UPDATE servers SET port = ?1, updated_at = ?2 WHERE id = ?3",
            params![port, now, id],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        update_count += 1;
    }
    if let Some(password) = &req.password {
        db.execute(
            "UPDATE servers SET password = ?1, updated_at = ?2 WHERE id = ?3",
            params![password, now, id],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        update_count += 1;
    }
    if let Some(max_players) = &req.max_players {
        db.execute(
            "UPDATE servers SET max_players = ?1, updated_at = ?2 WHERE id = ?3",
            params![max_players, now, id],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        update_count += 1;
    }
    if let Some(auto_start) = &req.auto_start {
        db.execute(
            "UPDATE servers SET auto_start = ?1, updated_at = ?2 WHERE id = ?3",
            params![*auto_start as i32, now, id],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        update_count += 1;
    }

    // If nothing was updated, just update the timestamp
    if update_count == 0 {
        db.execute(
            "UPDATE servers SET updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    // Fetch updated server
    let server = db
        .query_row(
            "SELECT id, name, port, tshock_version, world_name, status, password, max_players, auto_start, created_by, created_at, updated_at FROM servers WHERE id = ?1",
            params![id],
            |row| {
                Ok(Server {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    port: row.get(2)?,
                    tshock_version: row.get(3)?,
                    world_name: row.get(4)?,
                    status: row.get(5)?,
                    password: row.get(6)?,
                    max_players: row.get(7)?,
                    auto_start: row.get(8)?,
                    created_by: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            },
        )
        .map_err(|_| AppError::NotFound("Server not found".to_string()))?;

    Ok(Json(server))
}

pub async fn delete_server(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden(
            "Only administrators can delete servers".to_string(),
        ));
    }

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    db.execute("DELETE FROM servers WHERE id = ?1", params![id])
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Delete server directory
    let server_dir = state.config.server.data_dir.join("servers").join(&id);
    if server_dir.exists() {
        std::fs::remove_dir_all(server_dir)
            .map_err(|e| AppError::FileError(e.to_string()))?;
    }

    Ok(Json(json!({
        "success": true,
        "message": "Server deleted successfully"
    })))
}

pub async fn start_server(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can start servers".to_string(),
        ));
    }

    // Use a block to ensure MutexGuard is dropped before any .await
    let (port, max_players, password, tshock_version) = {
        let db = state.db.lock().map_err(|_| {
            AppError::InternalServerError("Failed to acquire database lock".to_string())
        })?;

        let result: (u16, i32, Option<String>, String) = db
            .query_row(
                "SELECT port, max_players, password, tshock_version FROM servers WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                    ))
                },
            )
            .map_err(|_| AppError::NotFound("Server not found".to_string()))?;
        result
        // db (MutexGuard) drops here at end of block
    };

    let version_path = state
        .version_manager
        .get_version_path(&tshock_version)
        .ok_or_else(|| AppError::NotFound(format!("TShock version {} not found", tshock_version)))?;

    let config_path = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(&id)
        .join("tshock");

    state
        .process_manager
        .start_server(
            &id,
            version_path.to_str().unwrap(),
            config_path.to_str().unwrap(),
            port,
            max_players,
            &password,
        )
        .await?;

    // Update database status — block ensures MutexGuard doesn't leak
    {
        let db = state.db.lock().map_err(|_| {
            AppError::InternalServerError("Failed to acquire database lock".to_string())
        })?;

        let now = Utc::now().to_rfc3339();
        db.execute(
            "UPDATE servers SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params!["running", now, id],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    Ok(Json(json!({
        "success": true,
        "message": "Server started successfully"
    })))
}

pub async fn stop_server(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can stop servers".to_string(),
        ));
    }

    state.process_manager.stop_server(&id).await?;

    // Update database status — block ensures MutexGuard doesn't leak
    {
        let db = state.db.lock().map_err(|_| {
            AppError::InternalServerError("Failed to acquire database lock".to_string())
        })?;

        let now = Utc::now().to_rfc3339();
        db.execute(
            "UPDATE servers SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params!["stopped", now, id],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    Ok(Json(json!({
        "success": true,
        "message": "Server stopped successfully"
    })))
}

pub async fn restart_server(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _ = stop_server(State(state.clone()), auth.clone(), Path(id.clone())).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    start_server(State(state), auth, Path(id)).await
}

pub async fn send_command(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<String>,
    Json(req): Json<CommandRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can send commands".to_string(),
        ));
    }

    state.process_manager.send_command(&id, &req.command).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Command sent successfully"
    })))
}

pub async fn server_status(
    State(state): State<AppState>,
    _auth: Auth,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let is_running = state.process_manager.is_running(&id).await;
    let status = if is_running { "running" } else { "stopped" };

    Ok(Json(json!({
        "status": status,
        "running": is_running
    })))
}
