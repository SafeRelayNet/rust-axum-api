use std::error::Error;

use axum::extract::rejection::MatchedPathRejection;
use axum::http::StatusCode;
use axum::{response::IntoResponse, BoxError};
use http_body_util::LengthLimitError;
use tower::timeout::error::Elapsed;

use crate::domain::errors::DomainError;

pub fn map_domain_error_to_status(error: &DomainError) -> StatusCode {
    match error {
        DomainError::Validation(_) => StatusCode::BAD_REQUEST,
        DomainError::NotFound(_) => StatusCode::NOT_FOUND,
        DomainError::Conflict(_) => StatusCode::CONFLICT,
        DomainError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        DomainError::Persistence(_) => StatusCode::INTERNAL_SERVER_ERROR,
        DomainError::Infrastructure(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub async fn handle_global_error(error: BoxError) -> impl IntoResponse {
    if find_cause::<LengthLimitError>(&*error).is_some() {
        return StatusCode::PAYLOAD_TOO_LARGE;
    }

    if error.is::<Elapsed>() {
        return StatusCode::REQUEST_TIMEOUT;
    }

    if find_cause::<MatchedPathRejection>(&*error).is_some() {
        return StatusCode::NOT_FOUND;
    }

    StatusCode::INTERNAL_SERVER_ERROR
}

fn find_cause<T: Error + 'static>(error: &dyn Error) -> Option<&T> {
    let mut source: Option<&dyn Error> = error.source();
    while let Some(current_source) = source {
        if let Some(typed_error) = current_source.downcast_ref::<T>() {
            return Some(typed_error);
        }
        source = current_source.source();
    }

    None
}
