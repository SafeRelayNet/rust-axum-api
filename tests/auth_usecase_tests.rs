use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use my_axum_project::application::auth_usecase::AuthUseCase;
use my_axum_project::domain::auth::{AuthTokenClaims, UserAuth};
use my_axum_project::domain::errors::DomainError;
use my_axum_project::domain::ports::{TokenBlocklistStore, TokenService, UserRepository};
use uuid::Uuid;

#[derive(Default)]
struct MockUserRepository {
    stored_user: Option<UserAuth>,
    create_should_fail_conflict: bool,
}

#[async_trait]
impl UserRepository for MockUserRepository {
    async fn create_user(&self, email: &str, _password_hash: &str) -> Result<Uuid, DomainError> {
        if self.create_should_fail_conflict {
            return Err(DomainError::Conflict(
                "email already registered".to_string(),
            ));
        }

        let user_id: Uuid = Uuid::new_v4();
        let _email_used: &str = email;

        Ok(user_id)
    }

    async fn find_user_by_email(&self, email: &str) -> Result<Option<UserAuth>, DomainError> {
        match &self.stored_user {
            Some(user) if user.email == email => Ok(Some(user.clone())),
            _ => Ok(None),
        }
    }
}

#[derive(Default)]
struct MockTokenService {
    validate_should_fail: bool,
}

#[async_trait]
impl TokenService for MockTokenService {
    async fn issue_token(&self, _user_id: Uuid, _email: &str) -> Result<String, DomainError> {
        Ok("mock.jwt.token".to_string())
    }

    async fn validate_token(&self, _token: &str) -> Result<AuthTokenClaims, DomainError> {
        if self.validate_should_fail {
            return Err(DomainError::Unauthorized("invalid token".to_string()));
        }

        let now_seconds_i64: i64 = chrono::Utc::now().timestamp();

        let now_seconds: u64 = if now_seconds_i64 < 0 {
            0
        } else {
            now_seconds_i64 as u64
        };

        Ok(AuthTokenClaims {
            sub: Uuid::new_v4().to_string(),
            email: "feature-check@example.com".to_string(),
            iat: now_seconds,
            exp: now_seconds + 3600,
            jti: Uuid::new_v4().to_string(),
        })
    }
}

#[derive(Default)]
struct MockTokenBlocklistStore {
    revoked_tokens: Mutex<HashSet<String>>,
}

#[async_trait]
impl TokenBlocklistStore for MockTokenBlocklistStore {
    async fn revoke_token(&self, token: &str, _ttl_seconds: u64) -> Result<(), DomainError> {
        let mut guard: std::sync::MutexGuard<'_, HashSet<String>> = self
            .revoked_tokens
            .lock()
            .map_err(|_e| DomainError::Infrastructure("mutex poisoned".to_string()))?;
        guard.insert(token.to_string());

        Ok(())
    }

    async fn is_token_revoked(&self, token: &str) -> Result<bool, DomainError> {
        let guard: std::sync::MutexGuard<'_, HashSet<String>> = self
            .revoked_tokens
            .lock()
            .map_err(|_e| DomainError::Infrastructure("mutex poisoned".to_string()))?;

        Ok(guard.contains(token))
    }
}

fn build_usecase(
    user_repository: Arc<dyn UserRepository>,
    token_service: Arc<dyn TokenService>,
    token_blocklist_store: Arc<dyn TokenBlocklistStore>,
) -> AuthUseCase {
    AuthUseCase::new(user_repository, token_service, token_blocklist_store)
}

#[tokio::test]
async fn register_rejects_empty_email() {
    let usecase: AuthUseCase = build_usecase(
        Arc::new(MockUserRepository::default()),
        Arc::new(MockTokenService::default()),
        Arc::new(MockTokenBlocklistStore::default()),
    );

    let result: Result<Uuid, DomainError> = usecase.register("", "supersecret").await;

    assert!(matches!(result, Err(DomainError::Validation(_))));
}

#[tokio::test]
async fn register_rejects_short_password() {
    let usecase: AuthUseCase = build_usecase(
        Arc::new(MockUserRepository::default()),
        Arc::new(MockTokenService::default()),
        Arc::new(MockTokenBlocklistStore::default()),
    );

    let result: Result<Uuid, DomainError> = usecase.register("user@example.com", "short").await;

    assert!(matches!(result, Err(DomainError::Validation(_))));
}

#[tokio::test]
async fn login_rejects_unknown_user() {
    let usecase: AuthUseCase = build_usecase(
        Arc::new(MockUserRepository::default()),
        Arc::new(MockTokenService::default()),
        Arc::new(MockTokenBlocklistStore::default()),
    );

    let result: Result<String, DomainError> =
        usecase.login("missing@example.com", "supersecret").await;
    assert!(matches!(result, Err(DomainError::Unauthorized(_))));
}

#[tokio::test]
async fn login_rejects_wrong_password() {
    let password_hash: String = bcrypt::hash("correct-password", bcrypt::DEFAULT_COST)
        .expect("hash generation should work in test");

    let user_repository: MockUserRepository = MockUserRepository {
        stored_user: Some(UserAuth {
            id: Uuid::new_v4(),
            email: "user@example.com".to_string(),
            password_hash,
        }),
        create_should_fail_conflict: false,
    };

    let usecase: AuthUseCase = build_usecase(
        Arc::new(user_repository),
        Arc::new(MockTokenService::default()),
        Arc::new(MockTokenBlocklistStore::default()),
    );

    let result: Result<String, DomainError> =
        usecase.login("user@example.com", "wrong-password").await;
    assert!(matches!(result, Err(DomainError::Unauthorized(_))));
}

#[tokio::test]
async fn login_returns_token_on_success() {
    let password_hash: String = bcrypt::hash("supersecret", bcrypt::DEFAULT_COST)
        .expect("hash generation should work in test");

    let user_repository: MockUserRepository = MockUserRepository {
        stored_user: Some(UserAuth {
            id: Uuid::new_v4(),
            email: "feature-check@example.com".to_string(),
            password_hash,
        }),
        create_should_fail_conflict: false,
    };

    let usecase: AuthUseCase = build_usecase(
        Arc::new(user_repository),
        Arc::new(MockTokenService::default()),
        Arc::new(MockTokenBlocklistStore::default()),
    );

    let result: Result<String, DomainError> = usecase
        .login("feature-check@example.com", "supersecret")
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn logout_rejects_invalid_token_format() {
    let usecase: AuthUseCase = build_usecase(
        Arc::new(MockUserRepository::default()),
        Arc::new(MockTokenService {
            validate_should_fail: true,
        }),
        Arc::new(MockTokenBlocklistStore::default()),
    );

    let result: Result<(), DomainError> = usecase.logout("not-a-jwt").await;
    assert!(matches!(result, Err(DomainError::Validation(_))));
}

#[tokio::test]
async fn logout_rejects_when_token_already_revoked() {
    let blocklist_store: Arc<MockTokenBlocklistStore> =
        Arc::new(MockTokenBlocklistStore::default());
    blocklist_store
        .revoke_token("mock.jwt.token", 3600)
        .await
        .expect("revocation insert should work in test");

    let usecase: AuthUseCase = build_usecase(
        Arc::new(MockUserRepository::default()),
        Arc::new(MockTokenService::default()),
        blocklist_store,
    );

    let result: Result<(), DomainError> = usecase.logout("mock.jwt.token").await;
    assert!(matches!(result, Err(DomainError::NotFound(_))));
}

#[tokio::test]
async fn logout_revokes_valid_token() {
    let blocklist_store: Arc<MockTokenBlocklistStore> =
        Arc::new(MockTokenBlocklistStore::default());
    let usecase: AuthUseCase = build_usecase(
        Arc::new(MockUserRepository::default()),
        Arc::new(MockTokenService::default()),
        blocklist_store.clone(),
    );

    let result: Result<(), DomainError> = usecase.logout("mock.jwt.token").await;
    assert!(result.is_ok());

    let revoked: bool = blocklist_store
        .is_token_revoked("mock.jwt.token")
        .await
        .expect("revocation lookup should work in test");
    assert!(revoked);
}
