use async_trait::async_trait;
use redis::AsyncCommands;

use crate::database::RedisService;
use crate::domain::auth::SessionData;
use crate::domain::errors::DomainError;
use crate::domain::ports::SessionStore;

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
impl SessionStore for RedisSessionStore {
    async fn store_session(
        &self,
        session_token: &str,
        data: &SessionData,
        ttl_seconds: u64,
    ) -> Result<(), DomainError> {
        let mut connection: redis::aio::MultiplexedConnection = self
            .redis_service
            .get_connection()
            .await
            .map_err(|error: anyhow::Error| DomainError::Infrastructure(error.to_string()))?;

        let redis_key: String = format!("session:{session_token}");
        let session_json: String = serde_json::json!({
            "user_id": data.user_id,
            "email": data.email
        })
        .to_string();

        let ttl_u64: u64 = ttl_seconds;
        let ttl_u64_bounded: u64 = ttl_u64.min(u64::from(u32::MAX));
        let ttl: u32 = ttl_u64_bounded as u32;
        connection
            .set_ex::<String, String, ()>(redis_key, session_json, ttl as u64)
            .await
            .map_err(|error: redis::RedisError| DomainError::Infrastructure(error.to_string()))?;

        Ok(())
    }

    async fn delete_session(&self, session_token: &str) -> Result<(), DomainError> {
        let mut connection: redis::aio::MultiplexedConnection = self
            .redis_service
            .get_connection()
            .await
            .map_err(|error: anyhow::Error| DomainError::Infrastructure(error.to_string()))?;

        let redis_key: String = format!("session:{session_token}");
        connection
            .del::<String, ()>(redis_key)
            .await
            .map_err(|error: redis::RedisError| DomainError::Infrastructure(error.to_string()))?;

        Ok(())
    }
}
