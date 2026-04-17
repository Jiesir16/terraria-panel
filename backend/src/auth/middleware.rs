use crate::error::AppError;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

impl Auth {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }

    pub fn is_operator_or_admin(&self) -> bool {
        self.role == "admin" || self.role == "operator"
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Auth
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract Authorization header manually
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing authorization header".to_string()))?;

        // Extract Bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Unauthorized("Invalid authorization format".to_string()))?;

        let token_manager = parts
            .extensions
            .get::<std::sync::Arc<super::TokenManager>>()
            .cloned()
            .ok_or_else(|| AppError::InternalServerError("Token manager not found".to_string()))?;

        let claims = token_manager.verify(token)?;

        Ok(Auth {
            user_id: claims.user_id,
            username: claims.username,
            role: claims.role,
        })
    }
}
