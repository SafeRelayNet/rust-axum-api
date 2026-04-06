use async_trait::async_trait;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use crate::domain::auth::AuthTokenClaims;
use crate::domain::errors::DomainError;
use crate::domain::ports::TokenService;

#[derive(Clone)]
pub struct JwtTokenService {
    jwt_secret: String,
    jwt_exp_seconds: u64,
}

impl JwtTokenService {
    pub fn new(jwt_secret: String, jwt_exp_seconds: u64) -> Self {
        Self {
            jwt_secret,
            jwt_exp_seconds,
        }
    }
}

#[async_trait]
impl TokenService for JwtTokenService {
    async fn issue_token(&self, user_id: Uuid, email: &str) -> Result<String, DomainError> {
        let now_timestamp: i64 = chrono::Utc::now().timestamp();
        let now_seconds: u64 = if now_timestamp < 0 {
            0
        } else {
            now_timestamp as u64
        };

        let claims: AuthTokenClaims = AuthTokenClaims {
            sub: user_id.to_string(),
            email: email.to_string(),
            iat: now_seconds,
            exp: now_seconds.saturating_add(self.jwt_exp_seconds),
            jti: Uuid::new_v4().to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|error: jsonwebtoken::errors::Error| {
            DomainError::Infrastructure(error.to_string())
        })
    }

    async fn validate_token(&self, token: &str) -> Result<AuthTokenClaims, DomainError> {
        let validation: Validation = Validation::default();
        let decoded = decode::<AuthTokenClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|error: jsonwebtoken::errors::Error| {
            DomainError::Unauthorized(error.to_string())
        })?;

        Ok(decoded.claims)
    }
}
