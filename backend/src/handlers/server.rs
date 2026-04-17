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

async fn is_server_port_ready(port: u16) -> bool {
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};

    let targets = [format!("127.0.0.1:{}", port), format!("[::1]:{}", port)];
    for target in targets {
        if timeout(Duration::from_millis(500), TcpStream::connect(&target))
            .await
            .ok()
            .and_then(Result::ok)
            .is_some()
        {
            return true;
        }
    }

    false
}

async fn wait_for_server_ready(
    state: &AppState,
    server_id: &str,
    port: u16,
    timeout_secs: u64,
) -> Result<bool, AppError> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

    loop {
        if is_server_port_ready(port).await {
            return Ok(true);
        }

        if !state.process_manager.is_running(server_id).await {
            return Err(AppError::ProcessError(
                "服务器进程启动后立即退出，请检查后端日志（stderr）了解详情".to_string(),
            ));
        }

        if tokio::time::Instant::now() >= deadline {
            return Ok(false);
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

fn is_world_file_name(name: &str) -> bool {
    name.ends_with(".wld") || name.ends_with(".wld.bak") || name.ends_with(".bak")
}

fn normalize_world_name_for_runtime(world_name: &str) -> String {
    if world_name.ends_with(".wld") {
        return world_name.to_string();
    }

    if world_name.ends_with(".wld.bak") {
        return world_name.trim_end_matches(".bak").to_string();
    }

    if world_name.ends_with(".bak") {
        let base = world_name.trim_end_matches(".bak");
        if base.ends_with(".wld") {
            return base.to_string();
        }
        return format!("{}.wld", base);
    }

    format!("{}.wld", world_name)
}

fn resolve_world_path(world_dir: &std::path::Path, world_name: &str) -> Option<String> {
    if world_name.is_empty() {
        return None;
    }

    let normalized = normalize_world_name_for_runtime(world_name);
    let candidates = vec![world_dir.join(&normalized)];

    for candidate in candidates {
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }

    // If the DB still points to a backup file, restore it to the active .wld name.
    if world_name != normalized {
        let backup_path = world_dir.join(world_name);
        let restored_path = world_dir.join(&normalized);
        if backup_path.exists() && backup_path.is_file() {
            if std::fs::copy(&backup_path, &restored_path).is_ok() {
                tracing::info!(
                    source = %backup_path.display(),
                    restored = %restored_path.display(),
                    "Restored backup world file to active .wld before startup"
                );
                return Some(restored_path.to_string_lossy().to_string());
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir(world_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let Some(fname) = path.file_name().and_then(|f| f.to_str()) else {
                continue;
            };

            if fname.ends_with(".wld")
                && (fname == normalized || fname.starts_with(world_name))
            {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }

    None
}

fn map_world_width_to_autocreate(width: Option<u64>) -> u32 {
    match width.unwrap_or(6400) {
        w if w <= 4200 => 1,
        w if w <= 6400 => 2,
        _ => 3,
    }
}

fn write_server_config_file(
    server_dir: &std::path::Path,
    world_path: &Option<String>,
    world_name_for_create: &Option<String>,
    autocreate: Option<u32>,
    port: u16,
    max_players: i32,
    password: &Option<String>,
    difficulty: Option<u32>,
    seed: &Option<String>,
) -> Result<String, AppError> {
    let mut lines = Vec::new();
    lines.push(format!("worldpath={}", server_dir.join("world").display()));
    lines.push(format!("port={}", port));
    lines.push(format!("maxplayers={}", max_players));

    if let Some(password) = password {
        if !password.is_empty() {
            lines.push(format!("password={}", password));
        }
    }

    if let Some(world_path) = world_path {
        lines.push(format!("world={}", world_path));
    } else {
        let size = autocreate.unwrap_or(2);
        let world_name = world_name_for_create
            .as_ref()
            .map(|name| normalize_world_name_for_runtime(name))
            .unwrap_or_else(|| "world.wld".to_string());
        let target_world_path = server_dir.join("world").join(&world_name);
        lines.push(format!("world={}", target_world_path.display()));
        lines.push(format!("autocreate={}", size));
        lines.push(format!(
            "worldname={}",
            world_name.trim_end_matches(".wld")
        ));
        lines.push(format!("difficulty={}", difficulty.unwrap_or(0)));
        if let Some(seed) = seed {
            if !seed.is_empty() {
                lines.push(format!("seed={}", seed));
            }
        }
    }

    let server_config_path = server_dir.join("serverconfig.txt");
    std::fs::write(&server_config_path, lines.join("\n"))
        .map_err(|e| AppError::FileError(format!("Failed to write serverconfig.txt: {}", e)))?;

    Ok(server_config_path.to_string_lossy().to_string())
}

fn sync_tshock_runtime_config(
    config_dir: &std::path::Path,
    server_name: &str,
    port: u16,
    max_players: i32,
    password: &Option<String>,
) -> Result<(), AppError> {
    let config_path = config_dir.join("config.json");
    let mut config = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| AppError::FileError(format!("Failed to read config.json: {}", e)))?;
        serde_json::from_str::<serde_json::Value>(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if !config.is_object() {
        config = serde_json::json!({});
    }

    let obj = config
        .as_object_mut()
        .ok_or_else(|| AppError::BadRequest("Invalid TShock config format".to_string()))?;

    obj.insert("ServerPort".to_string(), serde_json::json!(port));
    obj.insert("MaxSlots".to_string(), serde_json::json!(max_players));
    obj.insert("ServerName".to_string(), serde_json::json!(server_name));
    obj.insert("UseServerName".to_string(), serde_json::json!(true));
    obj.insert(
        "ServerPassword".to_string(),
        serde_json::json!(password.clone().unwrap_or_default()),
    );

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| AppError::BadRequest(format!("Failed to serialize TShock config: {}", e)))?;
    std::fs::write(&config_path, content)
        .map_err(|e| AppError::FileError(format!("Failed to write config.json: {}", e)))?;

    Ok(())
}

fn load_servers_from_db(state: &AppState) -> Result<Vec<Server>, AppError> {
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

    drop(stmt);
    drop(db);

    Ok(servers)
}

fn load_server_detail_from_db(state: &AppState, id: &str) -> Result<ServerDetail, AppError> {
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

    Ok(ServerDetail {
        server,
        player_count: 0,
        uptime_seconds: 0,
    })
}

pub async fn list_servers(
    State(state): State<AppState>,
    _auth: Auth,
) -> Result<Json<Vec<Server>>, AppError> {
    tracing::debug!("Listing all servers");
    let mut servers = load_servers_from_db(&state)?;

    // Cross-check: fix stale "running" status for servers whose process has exited
    for server in &mut servers {
        if server.status == "running" && !state.process_manager.is_running(&server.id).await {
            server.status = "stopped".to_string();
            if let Ok(db) = state.db.lock() {
                let now = Utc::now().to_rfc3339();
                let _ = db.execute(
                    "UPDATE servers SET status = ?1, updated_at = ?2 WHERE id = ?3",
                    params!["stopped", now, server.id],
                );
            }
        }
    }

    tracing::debug!(count = servers.len(), "Listed servers");
    Ok(Json(servers))
}

pub async fn create_server(
    State(state): State<AppState>,
    auth: Auth,
    Json(req): Json<CreateServerRequest>,
) -> Result<Json<Server>, AppError> {
    tracing::info!(user = %auth.username, server_name = %req.name, "Creating new server");

    if !auth.is_operator_or_admin() {
        tracing::warn!(user = %auth.username, role = %auth.role, "Create server denied: insufficient permissions");
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
    tracing::debug!(server_id = %server_id, path = %server_dir.display(), "Creating server directories");
    std::fs::create_dir_all(server_dir.join("world"))
        .map_err(|e| AppError::FileError(e.to_string()))?;
    std::fs::create_dir_all(server_dir.join("ServerPlugins"))
        .map_err(|e| AppError::FileError(e.to_string()))?;
    std::fs::create_dir_all(server_dir.join("logs"))
        .map_err(|e| AppError::FileError(e.to_string()))?;
    std::fs::create_dir_all(server_dir.join("tshock"))
        .map_err(|e| AppError::FileError(e.to_string()))?;

    tracing::info!(
        server_id = %server_id,
        server_name = %req.name,
        port = port,
        version = %req.tshock_version,
        user = %auth.username,
        "Server created successfully"
    );

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
    let mut detail = load_server_detail_from_db(&state, &id)?;

    // Cross-check: if DB says running but process is not, fix the status
    let actually_running = state.process_manager.is_running(&id).await;
    if detail.server.status == "running" && !actually_running {
        detail.server.status = "stopped".to_string();
        // Also fix DB
        if let Ok(db) = state.db.lock() {
            let now = Utc::now().to_rfc3339();
            let _ = db.execute(
                "UPDATE servers SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params!["stopped", now, id],
            );
        }
    }

    Ok(Json(detail))
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
    if let Some(world_name) = &req.world_name {
        db.execute(
            "UPDATE servers SET world_name = ?1, updated_at = ?2 WHERE id = ?3",
            params![world_name, now, id],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        update_count += 1;
    }
    if let Some(tshock_version) = &req.tshock_version {
        db.execute(
            "UPDATE servers SET tshock_version = ?1, updated_at = ?2 WHERE id = ?3",
            params![tshock_version, now, id],
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
    tracing::info!(user = %auth.username, server_id = %id, "Deleting server");

    if !auth.is_admin() {
        tracing::warn!(user = %auth.username, role = %auth.role, server_id = %id, "Delete server denied: insufficient permissions");
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
        tracing::debug!(server_id = %id, path = %server_dir.display(), "Removing server directory");
        std::fs::remove_dir_all(server_dir)
            .map_err(|e| AppError::FileError(e.to_string()))?;
    }

    tracing::info!(user = %auth.username, server_id = %id, "Server deleted successfully");

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
    tracing::info!(user = %auth.username, server_id = %id, "Starting server");

    if !auth.is_operator_or_admin() {
        tracing::warn!(user = %auth.username, role = %auth.role, server_id = %id, "Start server denied: insufficient permissions");
        return Err(AppError::Forbidden(
            "Only operators and admins can start servers".to_string(),
        ));
    }

    // Use a block to ensure MutexGuard is dropped before any .await
    let (server_name, port, max_players, password, tshock_version, world_name) = {
        let db = state.db.lock().map_err(|_| {
            AppError::InternalServerError("Failed to acquire database lock".to_string())
        })?;

        let result: (String, u16, i32, Option<String>, String, Option<String>) = db
            .query_row(
                "SELECT name, port, max_players, password, tshock_version, world_name FROM servers WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .map_err(|_| AppError::NotFound("Server not found".to_string()))?;
        result
        // db (MutexGuard) drops here at end of block
    };

    tracing::info!(
        server_id = %id,
        server_name = %server_name,
        port = port,
        max_players = max_players,
        tshock_version = %tshock_version,
        has_password = password.is_some(),
        world_name = ?world_name,
        "Server config loaded, starting process"
    );

    let version_path = state
        .version_manager
        .get_version_path(&tshock_version)
        .ok_or_else(|| {
            tracing::error!(server_id = %id, version = %tshock_version, "TShock version not found locally");
            AppError::NotFound(format!("TShock version {} not found", tshock_version))
        })?;

    let has_self_contained = version_path.join("TShock.Server").exists();
    let has_dll = version_path.join("TShock.Server.dll").exists();
    tracing::info!(
        server_id = %id,
        version = %tshock_version,
        resolved_path = %version_path.display(),
        has_self_contained = has_self_contained,
        has_dll = has_dll,
        "Resolved TShock version path"
    );

    let server_dir = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(&id);

    // Ensure server directories exist (they should from create, but verify)
    let _ = std::fs::create_dir_all(server_dir.join("world"));
    let _ = std::fs::create_dir_all(server_dir.join("tshock"));
    let _ = std::fs::create_dir_all(server_dir.join("ServerPlugins"));
    let _ = std::fs::create_dir_all(server_dir.join("logs"));

    let config_path = server_dir.join("tshock");
    sync_tshock_runtime_config(&config_path, &server_name, port, max_players, &password)?;

    // Resolve world file path
    let world_dir = server_dir.join("world");
    let world_name_clone = world_name.clone();
    let world_path = world_name.and_then(|wn| {
        let resolved = resolve_world_path(&world_dir, &wn);
        if resolved.is_none() {
            tracing::warn!(server_id = %id, world_name = %wn, "World file not found in server directory");
        }
        resolved
    });

    // Read TShock config for autocreate settings (world size)
    let (autocreate, world_name_for_create, difficulty, seed) = if world_path.is_none() {
        // No existing world file — check config for autocreate
        let config_json_path = config_path.join("config.json");
        if config_json_path.exists() {
            let config_str = std::fs::read_to_string(&config_json_path).unwrap_or_default();
            let config: serde_json::Value = serde_json::from_str(&config_str).unwrap_or_default();
            let auto = config.get("auto_create").and_then(|v| v.as_bool()).unwrap_or(false);
            let width = config.get("world_width").and_then(|v| v.as_u64());
            let size = map_world_width_to_autocreate(width);
            let wn = config.get("world_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or(world_name_clone.clone())
                .or_else(|| Some(server_name.clone()));
            let difficulty = config
                .get("difficulty")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            let seed = config
                .get("seed")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if auto {
                (Some(size), wn, difficulty, seed)
            } else {
                (Some(size), wn, difficulty, seed)
            }
        } else {
            (Some(2), world_name_clone.or_else(|| Some(server_name.clone())), Some(0), None)
        }
    } else {
        (None, None, None, None)
    };

    let server_config_path = write_server_config_file(
        &server_dir,
        &world_path,
        &world_name_for_create,
        autocreate,
        port,
        max_players,
        &password,
        difficulty,
        &seed,
    )?;

    tracing::debug!(
        server_id = %id,
        version_path = %version_path.display(),
        config_path = %config_path.display(),
        server_config_path = %server_config_path,
        world_path = ?world_path,
        autocreate = ?autocreate,
        "Launching TShock process"
    );

    state
        .process_manager
        .start_server(
            &id,
            version_path.to_str().unwrap(),
            config_path.to_str().unwrap(),
            &Some(server_config_path),
            port,
            max_players,
            &password,
            &world_path,
            autocreate,
            &world_name_for_create,
        )
        .await?;

    let ready = wait_for_server_ready(&state, &id, port, 30).await?;

    // Update database status — block ensures MutexGuard doesn't leak
    {
        let db = state.db.lock().map_err(|_| {
            AppError::InternalServerError("Failed to acquire database lock".to_string())
        })?;

        let now = Utc::now().to_rfc3339();
        db.execute(
            "UPDATE servers SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![if ready { "running" } else { "starting" }, now, id],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    if ready {
        tracing::info!(user = %auth.username, server_id = %id, port = port, "Server started successfully");
    } else {
        tracing::warn!(
            user = %auth.username,
            server_id = %id,
            port = port,
            "Server process is running but port is not ready yet"
        );
    }

    Ok(Json(json!({
        "success": true,
        "message": if ready {
            "Server started successfully"
        } else {
            "Server process started, but world is still loading and port is not ready yet"
        },
        "ready": ready
    })))
}

pub async fn stop_server(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(user = %auth.username, server_id = %id, "Stopping server");

    if !auth.is_operator_or_admin() {
        tracing::warn!(user = %auth.username, role = %auth.role, server_id = %id, "Stop server denied: insufficient permissions");
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

    tracing::info!(user = %auth.username, server_id = %id, "Server stopped successfully");

    Ok(Json(json!({
        "success": true,
        "message": "Server stopped successfully"
    })))
}

pub async fn kill_server(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::warn!(user = %auth.username, server_id = %id, "Force killing server");

    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can kill servers".to_string(),
        ));
    }

    state.process_manager.kill_server(&id).await?;

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
        "message": "Server force kill signal sent"
    })))
}

pub async fn restart_server(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(user = %auth.username, server_id = %id, "Restarting server");
    let _ = stop_server(State(state.clone()), auth.clone(), Path(id.clone())).await?;
    tracing::debug!(server_id = %id, "Waiting 2s before restart");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    start_server(State(state), auth, Path(id)).await
}

pub async fn send_command(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<String>,
    Json(req): Json<CommandRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(user = %auth.username, server_id = %id, command = %req.command, "Sending command to server");

    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can send commands".to_string(),
        ));
    }

    state.process_manager.send_command(&id, &req.command).await?;

    tracing::debug!(server_id = %id, command = %req.command, "Command sent successfully");

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
    let (port, db_status): (u16, Option<String>) = {
        let db = state.db.lock().map_err(|_| {
            AppError::InternalServerError("Failed to acquire database lock".to_string())
        })?;
        db.query_row(
            "SELECT port, status FROM servers WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| AppError::NotFound("Server not found".to_string()))?
    };

    let process_running = state.process_manager.is_running(&id).await;
    let port_ready = if process_running {
        is_server_port_ready(port).await
    } else {
        false
    };
    let status = if port_ready {
        "running"
    } else if process_running {
        "starting"
    } else {
        "stopped"
    };

    Ok(Json(json!({
        "status": status,
        "running": port_ready,
        "process_running": process_running,
        "db_status": db_status
    })))
}

/// List .wld world files available in a server's world directory
pub async fn list_worlds(
    State(state): State<AppState>,
    _auth: Auth,
    Path(id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let world_dir = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(&id)
        .join("world");

    let mut worlds = Vec::new();

    if world_dir.exists() {
        let entries = std::fs::read_dir(&world_dir)
            .map_err(|e| AppError::FileError(format!("Failed to read world directory: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(fname) = path.file_name().and_then(|f| f.to_str()) {
                    if is_world_file_name(fname) {
                        let metadata = std::fs::metadata(&path).ok();
                        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                        let modified = metadata
                            .and_then(|m| m.modified().ok())
                            .map(|t| {
                                let dt: chrono::DateTime<chrono::Utc> = t.into();
                                dt.format("%Y-%m-%d %H:%M").to_string()
                            })
                            .unwrap_or_default();

                        worlds.push(json!({
                            "name": fname,
                            "size": size,
                            "modified": modified,
                            "is_backup": fname.ends_with(".bak")
                        }));
                    }
                }
            }
        }
    }

    // Sort: .wld first, then by name
    worlds.sort_by(|a, b| {
        let a_bak = a["is_backup"].as_bool().unwrap_or(false);
        let b_bak = b["is_backup"].as_bool().unwrap_or(false);
        a_bak.cmp(&b_bak).then_with(|| {
            a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
        })
    });

    Ok(Json(worlds))
}
