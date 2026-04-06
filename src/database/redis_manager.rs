use crate::config::environment::EnvironmentVariables;
use anyhow::{Context, Result};
use redis::Client;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone)]
pub struct RedisService {
    client: Client,
}

impl RedisService {
    pub fn new(env: Arc<EnvironmentVariables>) -> Result<Self> {
        let client: Client =
            Client::open(env.redis_url.as_ref()).context("Failed to create Redis client")?;
        Ok(Self { client })
    }

    pub async fn initialize(&self) -> Result<()> {
        let mut conn: redis::aio::MultiplexedConnection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("Failed to connect to Redis")?;

        // Simple ping to verify connection
        let _: () = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .context("Failed to ping Redis")?;

        info!("Redis connection established successfully");
        Ok(())
    }

    pub async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .context("Failed to get Redis multiplexed connection")
    }

    pub async fn shutdown(&self) {
        // Redis client handles connection pooling/dropping automatically.
        // No explicit shutdown required for the client itself.
        info!("Redis service shutdown (noop)");
    }
}
