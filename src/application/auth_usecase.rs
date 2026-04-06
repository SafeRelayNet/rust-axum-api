use std::sync::Arc;

use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;

use crate::domain::errors::DomainError;
use crate::domain::ports::{TokenBlocklistStore, TokenService, UserRepository};

#[derive(Clone)]
pub struct AuthUseCase {
    user_repository: Arc<dyn UserRepository>,
    token_service: Arc<dyn TokenService>,
    token_blocklist_store: Arc<dyn TokenBlocklistStore>,
}

impl AuthUseCase {
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        token_service: Arc<dyn TokenService>,
        token_blocklist_store: Arc<dyn TokenBlocklistStore>,
    ) -> Self {
        Self {
            user_repository,
            token_service,
            token_blocklist_store,
        }
    }

    pub async fn register(&self, email: &str, password: &str) -> Result<Uuid, DomainError> {
        let normalized_email: String = email.trim().to_lowercase();

        if normalized_email.is_empty() {
            return Err(DomainError::Validation("email cannot be empty".to_string()));
        }

        if password.len() < 8 {
            return Err(DomainError::Validation(
                "password must have at least 8 characters".to_string(),
            ));
        }

        let password_hash: String = hash(password.as_bytes(), DEFAULT_COST)
            .map_err(|error: bcrypt::BcryptError| DomainError::Infrastructure(error.to_string()))?;

        self.user_repository
            .create_user(&normalized_email, &password_hash)
            .await
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<String, DomainError> {
        let normalized_email: String = email.trim().to_lowercase();
        let user: Option<crate::domain::auth::UserAuth> = self
            .user_repository
            .find_user_by_email(&normalized_email)
            .await?;

        let found_user: crate::domain::auth::UserAuth =
            user.ok_or_else(|| DomainError::Unauthorized("invalid credentials".to_string()))?;
        let is_password_valid: bool = verify(password.as_bytes(), &found_user.password_hash)
            .map_err(|error: bcrypt::BcryptError| DomainError::Infrastructure(error.to_string()))?;

        if !is_password_valid {
            return Err(DomainError::Unauthorized("invalid credentials".to_string()));
        }

        self.token_service
            .issue_token(found_user.id, &found_user.email)
            .await
    }

    pub async fn logout(&self, token: &str) -> Result<(), DomainError> {
        let normalized_token: String = token.trim().to_string();
        if normalized_token.is_empty() {
            return Err(DomainError::Validation(
                "session token cannot be empty".to_string(),
            ));
        }

        let claims: crate::domain::auth::AuthTokenClaims = self
            .token_service
            .validate_token(&normalized_token)
            .await
            .map_err(|_error: DomainError| {
                DomainError::Validation("session token must be a valid JWT".to_string())
            })?;

        let is_revoked: bool = self
            .token_blocklist_store
            .is_token_revoked(&normalized_token)
            .await?;
        if is_revoked {
            return Err(DomainError::NotFound("session not found".to_string()));
        }

        let now_seconds_i64: i64 = chrono::Utc::now().timestamp();
        let now_seconds: u64 = if now_seconds_i64 < 0 {
            0
        } else {
            now_seconds_i64 as u64
        };
        if claims.exp <= now_seconds {
            return Err(DomainError::Unauthorized("token has expired".to_string()));
        }

        let remaining_ttl_seconds: u64 = claims.exp.saturating_sub(now_seconds);
        self.token_blocklist_store
            .revoke_token(&normalized_token, remaining_ttl_seconds)
            .await?;

        Ok(())
    }
}
