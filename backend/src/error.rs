use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    InternalServerError(String),
    InvalidToken,
    DatabaseError(String),
    FileError(String),
    ProcessError(String),
}

impl AppError {
    pub fn status(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::InvalidToken => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn message(&self) -> String {
        match self {
            AppError::BadRequest(msg) => msg.clone(),
            AppError::Unauthorized(msg) => msg.clone(),
            AppError::Forbidden(msg) => msg.clone(),
            AppError::NotFound(msg) => msg.clone(),
            AppError::Conflict(msg) => msg.clone(),
            AppError::InvalidToken => "Invalid or expired token".to_string(),
            AppError::InternalServerError(msg) => msg.clone(),
            AppError::DatabaseError(msg) => format!("Database error: {}", msg),
            AppError::FileError(msg) => format!("File error: {}", msg),
            AppError::ProcessError(msg) => format!("Process error: {}", msg),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for AppError {}

impl From<argon2::password_hash::Error> for AppError {
    fn from(err: argon2::password_hash::Error) -> Self {
        AppError::InternalServerError(format!("Password hash error: {}", err))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let message = self.message();

        // Log all errors so they appear in server-side tracing output
        match &self {
            AppError::BadRequest(_) => tracing::warn!(status = %status, error = %message, "Bad request"),
            AppError::Unauthorized(_) | AppError::InvalidToken => {
                tracing::warn!(status = %status, error = %message, "Unauthorized access attempt")
            }
            AppError::Forbidden(_) => tracing::warn!(status = %status, error = %message, "Forbidden access attempt"),
            AppError::NotFound(_) => tracing::warn!(status = %status, error = %message, "Resource not found"),
            AppError::Conflict(_) => tracing::warn!(status = %status, error = %message, "Resource conflict"),
            AppError::InternalServerError(_) | AppError::DatabaseError(_) | AppError::FileError(_) | AppError::ProcessError(_) => {
                tracing::error!(status = %status, error = %message, "Internal server error")
            }
        }

        let body = Json(json!({
            "error": message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::FileError(err.to_string())
    }
}
