use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Json, Request};
use axum::http::StatusCode;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::ops::Deref;
use validator::Validate;

use crate::infrastructure::web::response::HandlerResponse;

/// Extractor that combines JSON parsing and DTO validation.
///
/// This keeps HTTP input validation in the infrastructure/web adapter layer,
/// while domain/application remain framework-agnostic.
pub struct ValidatedJson<T>(pub T);

impl<T> Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = HandlerResponse;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(payload): Json<T> =
            Json::<T>::from_request(request, state)
                .await
                .map_err(|error: JsonRejection| {
                    HandlerResponse::new(StatusCode::BAD_REQUEST)
                        .message("Invalid request body")
                        .data(json!({ "error": error.body_text() }))
                })?;

        payload
            .validate()
            .map_err(|error: validator::ValidationErrors| {
                HandlerResponse::new(StatusCode::BAD_REQUEST)
                    .message("Validation failed")
                    .data(json!({ "error": error.to_string() }))
            })?;

        Ok(Self(payload))
    }
}
