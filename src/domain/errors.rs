use thiserror::Error;

/// Domain-level error taxonomy for business operations.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("resource already exists: {0}")]
    Conflict(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("persistence error: {0}")]
    Persistence(String),
    #[error("infrastructure error: {0}")]
    Infrastructure(String),
}
