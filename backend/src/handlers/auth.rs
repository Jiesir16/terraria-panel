use axum::{extract::State, Json};
use chrono::Utc;
use rusqlite::params;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{hash_password, verify_password, Auth},
    error::AppError,
    models::{ChangePasswordRequest, LoginRequest, LoginResponse, RegisterRequest, UserInfo},
};

#[derive(Clone)]
pub struct AppState {
    pub db: crate::db::DbPool,
    pub config: crate::config::Config,
    pub token_manager: std::sync::Arc<crate::auth::TokenManager>,
    pub process_manager: std::sync::Arc<crate::services::ProcessManager>,
    pub version_manager: std::sync::Arc<crate::services::VersionManager>,
    pub mod_manager: std::sync::Arc<crate::services::ModManager>,
    pub save_manager: std::sync::Arc<crate::services::SaveManager>,
    pub system_monitor: std::sync::Arc<tokio::sync::Mutex<crate::services::SystemMonitor>>,
    pub frp_manager: std::sync::Arc<crate::services::FrpManager>,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    tracing::info!(username = %req.username, "Login attempt");

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    // Query user by username
    let result = db.query_row(
        "SELECT id, username, password_hash, role FROM users WHERE username = ?1",
        params![req.username],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    );

    let (user_id, username, password_hash, role) = result.map_err(|_| {
        tracing::warn!(username = %req.username, "Login failed: user not found");
        AppError::Unauthorized("Invalid username or password".to_string())
    })?;

    // Verify password
    if !verify_password(&req.password, &password_hash) {
        tracing::warn!(username = %req.username, "Login failed: incorrect password");
        return Err(AppError::Unauthorized(
            "Invalid username or password".to_string(),
        ));
    }

    // Generate JWT token
    let token = state
        .token_manager
        .generate(user_id.clone(), username.clone(), role.clone())?;

    tracing::info!(username = %username, role = %role, "Login successful");

    Ok(Json(LoginResponse {
        token,
        user: UserInfo {
            id: user_id,
            username,
            role,
        },
    }))
}

pub async fn register(
    State(state): State<AppState>,
    auth: Auth,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(operator = %auth.username, new_user = %req.username, "User registration attempt");

    // Only admin can register new users
    if !auth.is_admin() {
        tracing::warn!(operator = %auth.username, role = %auth.role, "Registration denied: insufficient permissions");
        return Err(AppError::Forbidden(
            "Only administrators can register new users".to_string(),
        ));
    }

    if req.username.is_empty() || req.password.is_empty() {
        return Err(AppError::BadRequest(
            "Username and password cannot be empty".to_string(),
        ));
    }

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    // Check if user already exists
    let exists: bool = db
        .query_row(
            "SELECT COUNT(*) > 0 FROM users WHERE username = ?1",
            params![req.username],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if exists {
        tracing::warn!(new_user = %req.username, "Registration failed: username already exists");
        return Err(AppError::Conflict("Username already exists".to_string()));
    }

    // Hash password and create user
    let user_id = Uuid::new_v4().to_string();
    let password_hash = hash_password(&req.password)?;
    let now = Utc::now().to_rfc3339();

    db.execute(
        "INSERT INTO users (id, username, password_hash, role, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![user_id, req.username, password_hash, "viewer", now, now],
    )
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    tracing::info!(operator = %auth.username, new_user = %req.username, user_id = %user_id, "User registered successfully");

    Ok(Json(json!({
        "success": true,
        "message": "User registered successfully"
    })))
}

pub async fn change_password(
    State(state): State<AppState>,
    auth: Auth,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(user = %auth.username, "Password change attempt");

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    // Get current user's password hash
    let password_hash: String = db
        .query_row(
            "SELECT password_hash FROM users WHERE id = ?1",
            params![auth.user_id],
            |row| row.get(0),
        )
        .map_err(|_| AppError::NotFound("User not found".to_string()))?;

    // Verify old password
    if !verify_password(&req.old_password, &password_hash) {
        tracing::warn!(user = %auth.username, "Password change failed: old password incorrect");
        return Err(AppError::Unauthorized(
            "Old password is incorrect".to_string(),
        ));
    }

    // Hash new password
    let new_hash = hash_password(&req.new_password)?;
    let now = Utc::now().to_rfc3339();

    db.execute(
        "UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE id = ?3",
        params![new_hash, now, auth.user_id],
    )
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    tracing::info!(user = %auth.username, "Password changed successfully");

    Ok(Json(json!({
        "success": true,
        "message": "Password changed successfully"
    })))
}

pub async fn me(auth: Auth) -> Result<Json<UserInfo>, AppError> {
    Ok(Json(UserInfo {
        id: auth.user_id,
        username: auth.username,
        role: auth.role,
    }))
}
