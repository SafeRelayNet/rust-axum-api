use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::Router;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::domain::errors::DomainError;
use crate::infrastructure::state::AppState;
use crate::infrastructure::web::error::map_domain_error_to_status;
use crate::infrastructure::web::response::HandlerResponse;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub token: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> HandlerResponse {
    let result: Result<Uuid, DomainError> = state
        .auth_usecase
        .register(&payload.email, &payload.password)
        .await;

    match result {
        Ok(user_id) => HandlerResponse::new(StatusCode::CREATED)
            .message("User registered successfully")
            .data(json!({ "user_id": user_id })),
        Err(error) => HandlerResponse::new(map_domain_error_to_status(&error))
            .message("Registration failed")
            .data(json!({ "error": error.to_string() })),
    }
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> HandlerResponse {
    let result: Result<String, DomainError> = state
        .auth_usecase
        .login(&payload.email, &payload.password)
        .await;

    match result {
        Ok(token) => HandlerResponse::new(StatusCode::OK)
            .message("Login successful")
            .data(json!(AuthResponse { token })),
        Err(error) => HandlerResponse::new(map_domain_error_to_status(&error))
            .message("Login failed")
            .data(json!({ "error": error.to_string() })),
    }
}

pub async fn logout(
    State(state): State<AppState>,
    Json(payload): Json<LogoutRequest>,
) -> HandlerResponse {
    let result: Result<(), DomainError> = state.auth_usecase.logout(&payload.token).await;

    match result {
        Ok(()) => HandlerResponse::new(StatusCode::OK)
            .message("Logout successful")
            .data(json!({ "logout": true })),
        Err(error) => HandlerResponse::new(map_domain_error_to_status(&error))
            .message("Logout failed")
            .data(json!({ "error": error.to_string() })),
    }
}
