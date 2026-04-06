use std::sync::Arc;

use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;

use crate::domain::auth::SessionData;
use crate::domain::errors::DomainError;
use crate::domain::ports::{SessionStore, UserRepository};

#[derive(Clone)]
pub struct AuthUseCase {
    user_repository: Arc<dyn UserRepository>,
    session_store: Arc<dyn SessionStore>,
}

impl AuthUseCase {
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        session_store: Arc<dyn SessionStore>,
    ) -> Self {
        Self {
            user_repository,
            session_store,
        }
    }

    pub async fn register(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Uuid, DomainError> {
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

        let token: String = Uuid::new_v4().to_string();
        let session_data: SessionData = SessionData {
            user_id: found_user.id,
            email: found_user.email,
        };

        self.session_store
            .store_session(&token, &session_data, 24 * 60 * 60)
            .await?;

        Ok(token)
    }

    pub async fn logout(&self, token: &str) -> Result<(), DomainError> {
        let normalized_token: String = token.trim().to_string();
        if normalized_token.is_empty() {
            return Err(DomainError::Validation(
                "session token cannot be empty".to_string(),
            ));
        }

        self.session_store.delete_session(&normalized_token).await
    }
}
