use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;

use crate::{
    auth::Auth,
    error::AppError,
    models::ServerConfig,
    handlers::AppState,
};

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

    // Try reading panel-config.json first (our format), fallback to config.json
    let panel_config_path = config_dir.join("panel-config.json");
    let config_path = config_dir.join("config.json");

    if panel_config_path.exists() {
        let config_str = std::fs::read_to_string(&panel_config_path)
            .map_err(|e| AppError::FileError(e.to_string()))?;
        let config: ServerConfig = serde_json::from_str(&config_str)
            .map_err(|e| {
                tracing::error!(server_id = %server_id, error = %e, "Failed to parse panel-config.json");
                AppError::BadRequest(format!("Invalid JSON: {}", e))
            })?;
        return Ok(Json(config));
    }

    if config_path.exists() {
        // Try to parse as our format (backward compat)
        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| AppError::FileError(e.to_string()))?;
        if let Ok(config) = serde_json::from_str::<ServerConfig>(&config_str) {
            return Ok(Json(config));
        }
        tracing::debug!(server_id = %server_id, "config.json not in panel format, returning defaults");
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

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| AppError::FileError(e.to_string()))?;

    // Save our panel config as panel-config.json (for UI round-trip)
    let panel_config_path = config_dir.join("panel-config.json");
    let panel_json = serde_json::to_string_pretty(&config)
        .map_err(|e| AppError::BadRequest(format!("Invalid config: {}", e)))?;
    std::fs::write(&panel_config_path, panel_json)
        .map_err(|e| AppError::FileError(e.to_string()))?;

    // Also sync to TShock's config.json in { "Settings": { ... } } format
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
    let settings_obj = settings.as_object_mut().unwrap();
    config.apply_to_tshock_settings(settings_obj);

    let tshock_json = serde_json::to_string_pretty(&tshock_config)
        .map_err(|e| AppError::BadRequest(format!("Failed to serialize: {}", e)))?;
    std::fs::write(&tshock_config_path, tshock_json)
        .map_err(|e| AppError::FileError(e.to_string()))?;

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

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| AppError::FileError(e.to_string()))?;

    let config_path = config_dir.join("config.json");
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| AppError::BadRequest(format!("Invalid config: {}", e)))?;

    std::fs::write(&config_path, config_json)
        .map_err(|e| AppError::FileError(e.to_string()))?;

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

    let config_path = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(&server_id)
        .join("tshock")
        .join("config.json");

    if !config_path.exists() {
        tracing::debug!(server_id = %server_id, "Config not found, exporting defaults");
        return Ok(Json(ServerConfig::default()));
    }

    let config_str = std::fs::read_to_string(&config_path)
        .map_err(|e| AppError::FileError(e.to_string()))?;

    let config: ServerConfig = serde_json::from_str(&config_str)
        .map_err(|e| {
            tracing::error!(server_id = %server_id, error = %e, "Failed to parse config.json for export");
            AppError::BadRequest(format!("Invalid JSON: {}", e))
        })?;

    tracing::info!(server_id = %server_id, "Config exported successfully");
    Ok(Json(config))
}
