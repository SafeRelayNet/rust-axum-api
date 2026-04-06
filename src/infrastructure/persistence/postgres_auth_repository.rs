use async_trait::async_trait;
use sqlx::Row;
use uuid::Uuid;

use crate::domain::auth::UserAuth;
use crate::domain::errors::DomainError;
use crate::domain::ports::UserRepository;
use crate::infrastructure::database::DatabaseService;

#[derive(Clone)]
pub struct PostgresAuthRepository {
    database_service: DatabaseService,
}

impl PostgresAuthRepository {
    pub fn new(database_service: DatabaseService) -> Self {
        Self { database_service }
    }
}

#[async_trait]
impl UserRepository for PostgresAuthRepository {
    async fn create_user(&self, email: &str, password_hash: &str) -> Result<Uuid, DomainError> {
        let email_owned: String = email.to_string();
        let password_hash_owned: String = password_hash.to_string();

        let pool: &sqlx::PgPool = self
            .database_service
            .get_pool()
            .map_err(|error: anyhow::Error| DomainError::Persistence(error.to_string()))?;

        let row: sqlx::postgres::PgRow = sqlx::query(
            r#"
            INSERT INTO users (email, password_hash)
            VALUES ($1, $2)
            RETURNING id
            "#,
        )
        .bind(email_owned)
        .bind(password_hash_owned)
        .fetch_one(pool)
        .await
        .map_err(|error: sqlx::Error| match error {
            sqlx::Error::Database(database_error)
                if database_error.code().as_deref() == Some("23505") =>
            {
                DomainError::Conflict("email already registered".to_string())
            }
            _ => DomainError::Persistence(error.to_string()),
        })?;

        let user_id: Uuid = row.get::<Uuid, _>("id");
        Ok(user_id)
    }

    async fn find_user_by_email(&self, email: &str) -> Result<Option<UserAuth>, DomainError> {
        let email_owned: String = email.to_string();
        let pool: &sqlx::PgPool = self
            .database_service
            .get_pool()
            .map_err(|error: anyhow::Error| DomainError::Persistence(error.to_string()))?;

        let row: Option<sqlx::postgres::PgRow> = sqlx::query(
            r#"
            SELECT id, email, password_hash
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email_owned)
        .fetch_optional(pool)
        .await
        .map_err(|error: sqlx::Error| DomainError::Persistence(error.to_string()))?;

        let mapped_user: Option<UserAuth> = row.map(|row_value: sqlx::postgres::PgRow| UserAuth {
            id: row_value.get::<Uuid, _>("id"),
            email: row_value.get::<String, _>("email"),
            password_hash: row_value.get::<String, _>("password_hash"),
        });

        Ok(mapped_user)
    }
}
