use async_trait::async_trait;

use crate::domain::auth::{SessionData, UserAuth};
use crate::domain::errors::DomainError;
use uuid::Uuid;

/// Outbound port for user identity persistence operations.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Persists a new user in the primary users table.
    async fn create_user(
        &self,
        email: &str,
        password_hash: &str,
    ) -> Result<Uuid, DomainError>;

    /// Fetches a user identity by email credential.
    async fn find_user_by_email(&self, email: &str) -> Result<Option<UserAuth>, DomainError>;
}

/// Outbound port for storing session state.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Persists an authenticated session with a finite lifetime.
    async fn store_session(
        &self,
        session_token: &str,
        data: &SessionData,
        ttl_seconds: u64,
    ) -> Result<(), DomainError>;

    /// Deletes an existing session by token.
    async fn delete_session(&self, session_token: &str) -> Result<(), DomainError>;
}
