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
    let config_path = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(&server_id)
        .join("tshock")
        .join("config.json");

    if !config_path.exists() {
        return Ok(Json(ServerConfig::default()));
    }

    let config_str = std::fs::read_to_string(&config_path)
        .map_err(|e| AppError::FileError(e.to_string()))?;

    let config: ServerConfig = serde_json::from_str(&config_str)
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    Ok(Json(config))
}

pub async fn update_config(
    State(state): State<AppState>,
    auth: Auth,
    Path(server_id): Path<String>,
    Json(config): Json<ServerConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
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

    let config_path = config_dir.join("config.json");
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| AppError::BadRequest(format!("Invalid config: {}", e)))?;

    std::fs::write(&config_path, config_json)
        .map_err(|e| AppError::FileError(e.to_string()))?;

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
    let config_path = state
        .config
        .server
        .data_dir
        .join("servers")
        .join(&server_id)
        .join("tshock")
        .join("config.json");

    if !config_path.exists() {
        return Ok(Json(ServerConfig::default()));
    }

    let config_str = std::fs::read_to_string(&config_path)
        .map_err(|e| AppError::FileError(e.to_string()))?;

    let config: ServerConfig = serde_json::from_str(&config_str)
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    Ok(Json(config))
}
