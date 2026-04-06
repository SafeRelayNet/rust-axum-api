use std::sync::Arc;

use anyhow::Result;

use crate::application::auth_usecase::AuthUseCase;
use crate::config::environment::EnvironmentVariables;
use crate::infrastructure::cache::redis_session_store::RedisSessionStore;
use crate::infrastructure::database::{DatabaseService, RedisService};
use crate::infrastructure::persistence::postgres_auth_repository::PostgresAuthRepository;
use crate::infrastructure::security::jwt_token_service::JwtTokenService;

/// Central dependency container used by the web layer.
///
/// This keeps application wiring in one place: config, use cases, and
/// infrastructure services are created once and then injected through Axum state.
#[derive(Clone)]
pub struct AppState {
    /// Runtime configuration shared across infrastructure components.
    pub environment: Arc<EnvironmentVariables>,
    /// Application entry point for auth business flows.
    pub auth_usecase: Arc<AuthUseCase>,
    /// PostgreSQL service (single pool + schema initialization).
    pub database: DatabaseService,
    /// Redis service used by session storage adapters.
    pub redis: RedisService,
}

impl AppState {
    /// Builds the full runtime graph for the API.
    ///
    /// Startup order matters:
    /// 1) load/validate environment
    /// 2) initialize PostgreSQL
    /// 3) initialize Redis
    /// 4) wire repositories/stores into use cases
    pub async fn build() -> Result<Self> {
        let environment: EnvironmentVariables = EnvironmentVariables::load()?;
        let environment_arc: Arc<EnvironmentVariables> = Arc::new(environment);

        let database: DatabaseService = DatabaseService::new(environment_arc.clone());
        database.initialize().await?;

        let redis: RedisService = RedisService::new(environment_arc.clone())?;
        redis.initialize().await?;

        let auth_repository: Arc<PostgresAuthRepository> =
            Arc::new(PostgresAuthRepository::new(database.clone()));

        let token_blocklist_store: Arc<RedisSessionStore> =
            Arc::new(RedisSessionStore::new(redis.clone()));
        let token_service: Arc<JwtTokenService> = Arc::new(JwtTokenService::new(
            environment_arc.jwt_secret.to_string(),
            environment_arc.jwt_exp_seconds,
        ));

        let auth_usecase: Arc<AuthUseCase> = Arc::new(AuthUseCase::new(
            auth_repository,
            token_service,
            token_blocklist_store,
        ));

        Ok(Self {
            environment: environment_arc,
            auth_usecase,
            database,
            redis,
        })
    }

    /// Gracefully closes external resources during shutdown.
    ///
    /// This is called by the server shutdown signal handler to ensure
    /// connection pools are closed cleanly before process exit.
    pub async fn shutdown(&self) {
        self.database.shutdown().await;
        self.redis.shutdown().await;
    }
}
