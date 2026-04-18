use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;

use crate::{
    auth::Auth,
    error::AppError,
    models::{ServerConfig, SscConfig},
    handlers::AppState,
};

fn load_panel_config(config_dir: &std::path::Path) -> Result<Option<ServerConfig>, AppError> {
    let panel_config_path = config_dir.join("panel-config.json");
    if !panel_config_path.exists() {
        return Ok(None);
    }

    let config_str = std::fs::read_to_string(&panel_config_path)
        .map_err(|e| AppError::FileError(e.to_string()))?;
    let config = serde_json::from_str::<ServerConfig>(&config_str)
        .map_err(|e| AppError::BadRequest(format!("Invalid panel-config.json: {}", e)))?;
    Ok(Some(config))
}

fn load_tshock_config(config_dir: &std::path::Path) -> Result<Option<ServerConfig>, AppError> {
    let config_path = config_dir.join("config.json");
    if !config_path.exists() {
        return Ok(None);
    }

    let config_str = std::fs::read_to_string(&config_path)
        .map_err(|e| AppError::FileError(e.to_string()))?;

    if let Ok(config) = serde_json::from_str::<ServerConfig>(&config_str) {
        return Ok(Some(config));
    }

    let value = serde_json::from_str::<serde_json::Value>(&config_str)
        .map_err(|e| AppError::BadRequest(format!("Invalid config.json: {}", e)))?;

    Ok(ServerConfig::from_tshock_config_value(&value))
}

fn load_ssc_config(config_dir: &std::path::Path) -> Result<Option<SscConfig>, AppError> {
    let ssc_config_path = config_dir.join("sscconfig.json");
    if !ssc_config_path.exists() {
        return Ok(None);
    }

    let config_str = std::fs::read_to_string(&ssc_config_path)
        .map_err(|e| AppError::FileError(e.to_string()))?;
    match serde_json::from_str::<SscConfig>(&config_str) {
        Ok(value) => Ok(Some(value)),
        Err(e) => {
            tracing::warn!(
                path = %ssc_config_path.display(),
                error = %e,
                "Failed to parse sscconfig.json, falling back to defaults"
            );
            Ok(Some(SscConfig::default()))
        }
    }
}

fn merge_ssc_settings(
    mut config: ServerConfig,
    ssc_config: Option<&SscConfig>,
) -> ServerConfig {
    if let Some(ssc_config) = ssc_config {
        config.server_side_character = Some(ssc_config.enabled);
    }

    config
}

fn sync_ssc_config(config_dir: &std::path::Path, config: &ServerConfig) -> Result<(), AppError> {
    let Some(enabled) = config.server_side_character else {
        return Ok(());
    };

    let ssc_config_path = config_dir.join("sscconfig.json");
    let mut ssc_json_value = if ssc_config_path.exists() {
        let content = std::fs::read_to_string(&ssc_config_path)
            .map_err(|e| AppError::FileError(e.to_string()))?;
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(value) if value.is_object() => value,
            Ok(_) => serde_json::to_value(SscConfig::default())
                .map_err(|e| AppError::BadRequest(format!("Failed to build default sscconfig.json: {}", e)))?,
            Err(e) => {
                tracing::warn!(
                    path = %ssc_config_path.display(),
                    error = %e,
                    "Invalid sscconfig.json while syncing Enabled, preserving defaults for missing structure only"
                );
                serde_json::to_value(SscConfig::default())
                    .map_err(|err| AppError::BadRequest(format!("Failed to build default sscconfig.json: {}", err)))?
            }
        }
    } else {
        serde_json::to_value(SscConfig::default())
            .map_err(|e| AppError::BadRequest(format!("Failed to build default sscconfig.json: {}", e)))?
    };

    if let Some(obj) = ssc_json_value.as_object_mut() {
        obj.insert("Enabled".to_string(), json!(enabled));
    }

    let ssc_json = serde_json::to_string_pretty(&ssc_json_value)
        .map_err(|e| AppError::BadRequest(format!("Failed to serialize sscconfig.json: {}", e)))?;
    std::fs::write(&ssc_config_path, ssc_json)
        .map_err(|e| AppError::FileError(e.to_string()))?;

    Ok(())
}

fn save_config_files(config_dir: &std::path::Path, config: &ServerConfig) -> Result<(), AppError> {
    std::fs::create_dir_all(config_dir)
        .map_err(|e| AppError::FileError(e.to_string()))?;

    let panel_config_path = config_dir.join("panel-config.json");
    let panel_json = serde_json::to_string_pretty(config)
        .map_err(|e| AppError::BadRequest(format!("Invalid config: {}", e)))?;
    std::fs::write(&panel_config_path, panel_json)
        .map_err(|e| AppError::FileError(e.to_string()))?;

    let tshock_config_path = config_dir.join("config.json");
    let mut tshock_config = if tshock_config_path.exists() {
        let content = std::fs::read_to_string(&tshock_config_path)
            .map_err(|e| AppError::FileError(e.to_string()))?;
        serde_json::from_str::<serde_json::Value>(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    if !tshock_config.is_object() {
        tshock_config = json!({});
    }

    let settings = tshock_config
        .as_object_mut()
        .unwrap()
        .entry("Settings")
        .or_insert_with(|| json!({}));
    if !settings.is_object() {
        *settings = json!({});
    }

    config.apply_to_tshock_settings(settings.as_object_mut().unwrap());

    let tshock_json = serde_json::to_string_pretty(&tshock_config)
        .map_err(|e| AppError::BadRequest(format!("Failed to serialize: {}", e)))?;
    std::fs::write(&tshock_config_path, tshock_json)
        .map_err(|e| AppError::FileError(e.to_string()))?;

    sync_ssc_config(config_dir, config)?;

    Ok(())
}

fn sync_server_row_from_config(
    state: &AppState,
    server_id: &str,
    config: &ServerConfig,
) -> Result<(), AppError> {
    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    let now = chrono::Utc::now().to_rfc3339();
    db.execute(
        "UPDATE servers
         SET port = COALESCE(?1, port),
             password = COALESCE(?2, password),
             max_players = COALESCE(?3, max_players),
             world_name = COALESCE(?4, world_name),
             updated_at = ?5
         WHERE id = ?6",
        rusqlite::params![
            config.port,
            config.server_password,
            config.max_players,
            config.world_name,
            now,
            server_id
        ],
    )
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(())
}

pub async fn get_config(
    State(state): State<AppState>,
    _auth: Auth,
    Path(server_id): Path<String>,
) -> Result<Json<ServerConfig>, AppError> {
    tracing::debug!(server_id = %server_id, "Reading server config");

    let config_dir = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(&server_id)
        .join("tshock");

    let ssc_config = load_ssc_config(&config_dir)?;

    if let Some(config) = load_panel_config(&config_dir)? {
        return Ok(Json(merge_ssc_settings(config, ssc_config.as_ref())));
    }

    if let Some(config) = load_tshock_config(&config_dir)? {
        return Ok(Json(merge_ssc_settings(config, ssc_config.as_ref())));
    }

    tracing::debug!(server_id = %server_id, "Config file not found, returning defaults");
    Ok(Json(ServerConfig::default()))
}

pub async fn update_config(
    State(state): State<AppState>,
    auth: Auth,
    Path(server_id): Path<String>,
    Json(config): Json<ServerConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(user = %auth.username, server_id = %server_id, "Updating server config");

    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can update config".to_string(),
        ));
    }

    let config_dir = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(&server_id)
        .join("tshock");

    save_config_files(&config_dir, &config)?;
    sync_server_row_from_config(&state, &server_id, &config)?;

    tracing::info!(server_id = %server_id, "Config updated (panel + TShock config.json)");

    crate::db::log_operation(&state.db, &auth.user_id, "更新配置", Some(&server_id), None);

    Ok(Json(json!({
        "success": true,
        "message": "Config updated successfully"
    })))
}

pub async fn list_templates(_auth: Auth) -> Result<Json<Vec<crate::models::ServerConfigTemplate>>, AppError> {
    let templates = crate::models::get_templates();
    Ok(Json(templates))
}

pub async fn import_config(
    State(state): State<AppState>,
    auth: Auth,
    Path(server_id): Path<String>,
    Json(config): Json<ServerConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(user = %auth.username, server_id = %server_id, "Importing config");

    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can import config".to_string(),
        ));
    }

    let config_dir = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(&server_id)
        .join("tshock");

    save_config_files(&config_dir, &config)?;
    sync_server_row_from_config(&state, &server_id, &config)?;

    tracing::info!(server_id = %server_id, "Config imported successfully");

    Ok(Json(json!({
        "success": true,
        "message": "Config imported successfully"
    })))
}

pub async fn export_config(
    State(state): State<AppState>,
    _auth: Auth,
    Path(server_id): Path<String>,
) -> Result<Json<ServerConfig>, AppError> {
    tracing::info!(server_id = %server_id, "Exporting server config");

    let config_dir = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(&server_id)
        .join("tshock");

    let ssc_config = load_ssc_config(&config_dir)?;
    let config = load_panel_config(&config_dir)?
        .or(load_tshock_config(&config_dir)?)
        .unwrap_or_default();

    tracing::info!(server_id = %server_id, "Config exported successfully");
    Ok(Json(merge_ssc_settings(config, ssc_config.as_ref())))
}

pub async fn get_ssc_config(
    State(state): State<AppState>,
    _auth: Auth,
    Path(server_id): Path<String>,
) -> Result<Json<SscConfig>, AppError> {
    let config_dir = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(&server_id)
        .join("tshock");

    Ok(Json(load_ssc_config(&config_dir)?.unwrap_or_default()))
}

pub async fn update_ssc_config(
    State(state): State<AppState>,
    auth: Auth,
    Path(server_id): Path<String>,
    Json(ssc_config): Json<SscConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth.is_operator_or_admin() {
        return Err(AppError::Forbidden(
            "Only operators and admins can update SSC config".to_string(),
        ));
    }

    let config_dir = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(&server_id)
        .join("tshock");

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| AppError::FileError(e.to_string()))?;

    let ssc_config_path = config_dir.join("sscconfig.json");
    let ssc_json = serde_json::to_string_pretty(&ssc_config)
        .map_err(|e| AppError::BadRequest(format!("Invalid SSC config: {}", e)))?;
    std::fs::write(&ssc_config_path, ssc_json)
        .map_err(|e| AppError::FileError(e.to_string()))?;

    let mut merged_config = load_panel_config(&config_dir)?
        .or(load_tshock_config(&config_dir)?)
        .unwrap_or_default();
    merged_config.server_side_character = Some(ssc_config.enabled);
    save_config_files(&config_dir, &merged_config)?;
    sync_server_row_from_config(&state, &server_id, &merged_config)?;

    crate::db::log_operation(&state.db, &auth.user_id, "更新SSC配置", Some(&server_id), None);

    Ok(Json(json!({
        "success": true,
        "message": "SSC config updated successfully"
    })))
}
