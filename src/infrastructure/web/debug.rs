use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use serde_json::json;
use tokio::time::{sleep, Duration};

use crate::infrastructure::state::AppState;
use crate::infrastructure::web::response::HandlerResponse;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/debug/sleep/{seconds}", get(sleep_handler))
        .route("/debug/error", get(error_handler))
}

async fn sleep_handler(Path(seconds): Path<u64>) -> HandlerResponse {
    sleep(Duration::from_secs(seconds)).await;

    HandlerResponse::new(StatusCode::OK)
        .message("Sleep handler completed")
        .data(json!({ "slept_seconds": seconds }))
}

async fn error_handler() -> HandlerResponse {
    HandlerResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
        .message("Debug internal error")
        .data(json!({ "error": "debug_internal_error" }))
}
