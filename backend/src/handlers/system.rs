use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use rusqlite::params;
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;

use crate::{
    auth::Auth,
    error::AppError,
    models::UserInfo,
    handlers::AppState,
};

#[derive(Debug, Serialize)]
pub struct OperationLog {
    pub id: i64,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub action: String,
    pub target: Option<String>,
    pub details: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub password: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: String,
}

pub async fn system_info(
    State(state): State<AppState>,
    _auth: Auth,
) -> Result<Json<crate::services::system_monitor::SystemInfo>, AppError> {
    let mut system_monitor = state.system_monitor.lock().await;
    let info = system_monitor.get_system_info();
    Ok(Json(info))
}

pub async fn list_logs(
    State(state): State<AppState>,
    _auth: Auth,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<OperationLog>>, AppError> {
    let limit: i64 = params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);
    let offset: i64 = params
        .get("offset")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    let mut stmt = db
        .prepare(
            "SELECT l.id, l.user_id, u.username, l.action, l.target, l.details, l.created_at
             FROM operation_logs l
             LEFT JOIN users u ON u.id = l.user_id
             ORDER BY l.created_at DESC
             LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let logs = stmt
        .query_map(params![limit, offset], |row| {
            Ok(OperationLog {
                id: row.get(0)?,
                user_id: row.get(1)?,
                username: row.get(2)?,
                action: row.get(3)?,
                target: row.get(4)?,
                details: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(Json(logs))
}

pub async fn list_users(
    State(state): State<AppState>,
    auth: Auth,
) -> Result<Json<Vec<UserInfo>>, AppError> {
    if !auth.is_admin() {
        return Err(AppError::Forbidden(
            "Only administrators can list users".to_string(),
        ));
    }

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    let mut stmt = db
        .prepare("SELECT id, username, role FROM users")
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let users = stmt
        .query_map([], |row| {
            Ok(UserInfo {
                id: row.get(0)?,
                username: row.get(1)?,
                role: row.get(2)?,
            })
        })
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(Json(users))
}

pub async fn create_user(
    State(state): State<AppState>,
    auth: Auth,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(admin = %auth.username, new_user = %req.username, "Admin creating new user");

    if !auth.is_admin() {
        tracing::warn!(user = %auth.username, role = %auth.role, "Create user denied: insufficient permissions");
        return Err(AppError::Forbidden(
            "Only administrators can create users".to_string(),
        ));
    }

    if req.username.is_empty() || req.password.is_empty() {
        return Err(AppError::BadRequest(
            "Username and password cannot be empty".to_string(),
        ));
    }

    if !matches!(req.role.as_str(), "admin" | "operator" | "viewer") {
        return Err(AppError::BadRequest(
            "Invalid role. Expected one of: admin, operator, viewer".to_string(),
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
        tracing::warn!(new_user = %req.username, "User creation failed: username already exists");
        return Err(AppError::Conflict(
            "Username already exists".to_string(),
        ));
    }

    let user_id = Uuid::new_v4().to_string();
    let password_hash = crate::auth::hash_password(&req.password)?;
    let now = Utc::now().to_rfc3339();

    db.execute(
        "INSERT INTO users (id, username, password_hash, role, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![user_id, req.username, password_hash, req.role, now, now],
    )
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    tracing::info!(admin = %auth.username, new_user = %req.username, user_id = %user_id, "User created successfully");
    drop(db);
    crate::db::log_operation(&state.db, &auth.user_id, "创建用户", Some(&req.username), Some(&format!("role={}", req.role)));

    Ok(Json(json!({
        "success": true,
        "message": "User created successfully"
    })))
}

pub async fn update_user(
    State(state): State<AppState>,
    auth: Auth,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(admin = %auth.username, target_user_id = %user_id, "Admin updating user");

    if !auth.is_admin() {
        tracing::warn!(user = %auth.username, role = %auth.role, "Update user denied: insufficient permissions");
        return Err(AppError::Forbidden(
            "Only administrators can update users".to_string(),
        ));
    }

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    let now = Utc::now().to_rfc3339();

    if let Some(username) = &req.username {
        tracing::debug!(target_user_id = %user_id, new_username = %username, "Updating username");
        db.execute(
            "UPDATE users SET username = ?1, updated_at = ?2 WHERE id = ?3",
            params![username, now, user_id],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    if let Some(_password) = &req.password {
        tracing::debug!(target_user_id = %user_id, "Updating password");
        let password_hash = crate::auth::hash_password(_password)?;
        db.execute(
            "UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE id = ?3",
            params![password_hash, now, user_id],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    if let Some(role) = &req.role {
        tracing::debug!(target_user_id = %user_id, new_role = %role, "Updating role");
        db.execute(
            "UPDATE users SET role = ?1, updated_at = ?2 WHERE id = ?3",
            params![role, now, user_id],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    tracing::info!(admin = %auth.username, target_user_id = %user_id, "User updated successfully");
    drop(db);
    crate::db::log_operation(&state.db, &auth.user_id, "更新用户", Some(&user_id), None);

    Ok(Json(json!({
        "success": true,
        "message": "User updated successfully"
    })))
}

pub async fn delete_user(
    State(state): State<AppState>,
    auth: Auth,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(admin = %auth.username, target_user_id = %user_id, "Admin deleting user");

    if !auth.is_admin() {
        tracing::warn!(user = %auth.username, role = %auth.role, "Delete user denied: insufficient permissions");
        return Err(AppError::Forbidden(
            "Only administrators can delete users".to_string(),
        ));
    }

    // Prevent deleting the current user
    if user_id == auth.user_id {
        tracing::warn!(admin = %auth.username, "Delete user denied: cannot delete own account");
        return Err(AppError::BadRequest(
            "Cannot delete your own user account".to_string(),
        ));
    }

    let db = state.db.lock().map_err(|_| {
        AppError::InternalServerError("Failed to acquire database lock".to_string())
    })?;

    db.execute("DELETE FROM users WHERE id = ?1", params![user_id])
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    drop(db);
    crate::db::log_operation(&state.db, &auth.user_id, "删除用户", Some(&user_id), None);

    tracing::info!(admin = %auth.username, target_user_id = %user_id, "User deleted successfully");

    Ok(Json(json!({
        "success": true,
        "message": "User deleted successfully"
    })))
}
