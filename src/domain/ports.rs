use async_trait::async_trait;

use crate::domain::auth::{AuthTokenClaims, UserAuth};
use crate::domain::errors::DomainError;
use uuid::Uuid;

/// Outbound port for user identity persistence operations.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Persists a new user in the primary users table.
    async fn create_user(&self, email: &str, password_hash: &str) -> Result<Uuid, DomainError>;

    /// Fetches a user identity by email credential.
    async fn find_user_by_email(&self, email: &str) -> Result<Option<UserAuth>, DomainError>;
}

/// Outbound port for JWT generation and verification.
#[async_trait]
pub trait TokenService: Send + Sync {
    /// Issues a signed JWT token for a user.
    async fn issue_token(&self, user_id: Uuid, email: &str) -> Result<String, DomainError>;

    /// Validates and decodes a JWT token.
    async fn validate_token(&self, token: &str) -> Result<AuthTokenClaims, DomainError>;
}

/// Outbound port for token revocation storage.
#[async_trait]
pub trait TokenBlocklistStore: Send + Sync {
    /// Marks a token as revoked for the remaining token lifetime.
    async fn revoke_token(&self, token: &str, ttl_seconds: u64) -> Result<(), DomainError>;

    /// Checks whether a token is currently revoked.
    async fn is_token_revoked(&self, token: &str) -> Result<bool, DomainError>;
}
