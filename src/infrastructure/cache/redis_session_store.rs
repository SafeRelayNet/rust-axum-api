use async_trait::async_trait;
use redis::AsyncCommands;

use crate::domain::errors::DomainError;
use crate::domain::ports::TokenBlocklistStore;
use crate::infrastructure::database::RedisService;

#[derive(Clone)]
pub struct RedisSessionStore {
    redis_service: RedisService,
}

impl RedisSessionStore {
    pub fn new(redis_service: RedisService) -> Self {
        Self { redis_service }
    }
}

#[async_trait]
impl TokenBlocklistStore for RedisSessionStore {
    async fn revoke_token(&self, token: &str, ttl_seconds: u64) -> Result<(), DomainError> {
        let mut connection: redis::aio::MultiplexedConnection = self
            .redis_service
            .get_connection()
            .await
            .map_err(|error: anyhow::Error| DomainError::Infrastructure(error.to_string()))?;

        let redis_key: String = format!("revoked_jwt:{token}");

        let ttl_u64: u64 = ttl_seconds;
        let ttl_u64_bounded: u64 = ttl_u64.min(u64::from(u32::MAX));
        let ttl: u32 = ttl_u64_bounded as u32;
        connection
            .set_ex::<String, String, ()>(redis_key, "1".to_string(), ttl as u64)
            .await
            .map_err(|error: redis::RedisError| DomainError::Infrastructure(error.to_string()))?;

        Ok(())
    }

    async fn is_token_revoked(&self, token: &str) -> Result<bool, DomainError> {
        let mut connection: redis::aio::MultiplexedConnection = self
            .redis_service
            .get_connection()
            .await
            .map_err(|error: anyhow::Error| DomainError::Infrastructure(error.to_string()))?;

        let redis_key: String = format!("revoked_jwt:{token}");
        let exists: bool = connection
            .exists::<String, bool>(redis_key)
            .await
            .map_err(|error: redis::RedisError| DomainError::Infrastructure(error.to_string()))?;

        Ok(exists)
    }
}
