// =============================================================================
// DATABASE SERVICE - Single PostgreSQL pool
// =============================================================================

use anyhow::{Context, Result};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, Executor, PgPool};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{debug, info, log::LevelFilter};

use crate::config::environment::EnvironmentVariables;

// =============================================================================
// SQL CONSTANTS
// =============================================================================

/// Single initialization SQL script
const INIT_SCHEMA_SQL: &str = include_str!("sql/schemas/schema_init.sql");

// =============================================================================
// DATABASE SERVICE
// =============================================================================

/// Database service managing a single PostgreSQL connection pool.
#[derive(Clone, Debug)]
pub struct DatabaseService {
    /// Single connection pool for the application
    pool: Arc<OnceCell<PgPool>>,
    /// Environment configuration
    config: Arc<EnvironmentVariables>,
}

impl DatabaseService {
    /// Creates a new DatabaseService instance.
    /// Note: The pool is not initialized until `initialize()` is called.
    pub fn new(config: Arc<EnvironmentVariables>) -> Self {
        Self {
            pool: Arc::new(OnceCell::new()),
            config,
        }
    }

    /// Initializes the database service by creating the pool and running migrations.
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing DatabaseService...");

        // Initialize the pool if not already initialized
        self.pool
            .get_or_try_init(|| async { self.create_pool().await })
            .await?;

        // Get reference to the pool
        let pool: &sqlx::Pool<sqlx::Postgres> = self.get_pool()?;

        // Run schema initialization
        self.initialize_schema(pool).await?;

        info!("DatabaseService initialized successfully");
        Ok(())
    }

    /// Gracefully shuts down the service.
    pub async fn shutdown(&self) {
        info!("Initiating DatabaseService shutdown...");
        if let Some(pool) = self.pool.get() {
            pool.close().await;
            info!("Database connection pool closed");
        } else {
            debug!("Database pool was not initialized, nothing to close");
        }
    }

    /// Returns the connection pool.
    /// Errors if the pool has not been initialized.
    pub fn get_pool(&self) -> Result<&PgPool> {
        self.pool
            .get()
            .ok_or_else(|| anyhow::anyhow!("Database pool not initialized"))
    }
}

// =============================================================================
// INTERNAL HELPERS
// =============================================================================

impl DatabaseService {
    /// Creates the connection pool based on environment config
    async fn create_pool(&self) -> Result<PgPool> {
        let connect_options: PgConnectOptions = self.create_connect_options().await?;

        let pool: sqlx::Pool<sqlx::Postgres> = PgPoolOptions::new()
            .max_connections(20) // Adjust based on load/Cloud Run specs
            .min_connections(5)
            .idle_timeout(std::time::Duration::from_secs(30))
            .connect_with(connect_options)
            .await
            .context("Failed to create database connection pool")?;

        Ok(pool)
    }

    /// Creates connection options with SSL and UTC timezone
    async fn create_connect_options(&self) -> Result<PgConnectOptions> {
        let mut options: PgConnectOptions = PgConnectOptions::new()
            .host(&self.config.db_host)
            .port(self.config.db_port)
            .username(&self.config.db_user)
            .password(&self.config.db_password)
            .database(&self.config.db_name)
            .log_statements(LevelFilter::Debug);

        // Always use UTC and standard app name
        options = options.options([
            ("timezone", "UTC"),
            ("application_name", "rust-axum-api-rls"),
        ]);

        // Configure SSL based on environment
        let is_development: bool = self.config.environment == "development";
        if !is_development {
            options = options.ssl_mode(sqlx::postgres::PgSslMode::Require);
        } else {
            options = options.ssl_mode(sqlx::postgres::PgSslMode::Prefer);
        }

        Ok(options)
    }

    /// Runs the initialization SQL
    async fn initialize_schema(&self, pool: &PgPool) -> Result<()> {
        info!("Executing schema initialization...");

        pool.execute(INIT_SCHEMA_SQL)
            .await
            .context("Failed to execute schema initialization SQL")?;

        info!("Schema initialization completed");
        Ok(())
    }
}
