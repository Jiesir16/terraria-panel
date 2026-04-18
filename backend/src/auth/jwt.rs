use crate::error::AppError;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub username: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Clone)]
pub struct TokenManager {
    secret: String,
    expire_hours: u64,
}

impl TokenManager {
    pub fn new(secret: String, expire_hours: u64) -> Self {
        Self {
            secret,
            expire_hours,
        }
    }

    pub fn generate(
        &self,
        user_id: String,
        username: String,
        role: String,
    ) -> Result<String, AppError> {
        let now = Utc::now();
        let expire = now + Duration::hours(self.expire_hours as i64);

        let claims = Claims {
            user_id,
            username,
            role,
            iat: now.timestamp(),
            exp: expire.timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
        .map_err(|_| AppError::InternalServerError("Failed to generate token".to_string()))
    }

    pub fn verify(&self, token: &str) -> Result<Claims, AppError> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|_| AppError::InvalidToken)
    }
}
