use std::time::Duration;

use axum::body::{to_bytes, Body, Bytes};
use axum::error_handling::HandleErrorLayer;
use axum::extract::DefaultBodyLimit;
use axum::http::{Method, Request, StatusCode};
use axum::middleware::from_fn;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use my_axum_project::infrastructure::web::error::handle_global_error;
use my_axum_project::infrastructure::web::response::response_wrapper;
use serde_json::Value;
use tower::timeout::TimeoutLayer;
use tower::{ServiceBuilder, ServiceExt};

async fn ok_handler() -> StatusCode {
    StatusCode::OK
}

async fn sleep_handler() -> StatusCode {
    tokio::time::sleep(Duration::from_millis(120)).await;
    StatusCode::OK
}

async fn echo_handler(_body: Bytes) -> StatusCode {
    StatusCode::OK
}

fn feature_router() -> Router {
    Router::new()
        .route("/ok", get(ok_handler))
        .route("/sleep", get(sleep_handler))
        .route("/echo", post(echo_handler))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_global_error))
                .layer(TimeoutLayer::new(Duration::from_millis(40))),
        )
        .layer(from_fn(response_wrapper))
        .layer(DefaultBodyLimit::max(16))
}

#[tokio::test]
async fn response_wrapper_returns_standard_envelope() {
    let app: Router = feature_router();

    let request: Request<Body> = Request::builder()
        .method(Method::GET)
        .uri("/ok")
        .body(Body::empty())
        .expect("request build should succeed");

    let response: axum::http::Response<Body> = app
        .oneshot(request)
        .await
        .expect("request should be handled");

    assert_eq!(response.status(), StatusCode::OK);

    let bytes: Bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");

    let json_body: Value = serde_json::from_slice(&bytes).expect("body should be valid JSON");

    assert!(json_body.get("status").is_some());
    assert!(json_body.get("code").is_some());
    assert!(json_body.get("data").is_some());
    assert!(json_body.get("messages").is_some());
    assert!(json_body.get("date").is_some());
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app: Router = feature_router();

    let request: Request<Body> = Request::builder()
        .method(Method::GET)
        .uri("/missing")
        .body(Body::empty())
        .expect("request build should succeed");

    let response: axum::http::Response<Body> = app
        .oneshot(request)
        .await
        .expect("request should be handled");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn timeout_route_returns_408() {
    let app: Router = feature_router();

    let request: Request<Body> = Request::builder()
        .method(Method::GET)
        .uri("/sleep")
        .body(Body::empty())
        .expect("request build should succeed");

    let response: axum::http::Response<Body> = app
        .oneshot(request)
        .await
        .expect("request should be handled");

    assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
}

#[tokio::test]
async fn payload_too_large_returns_413() {
    let app: Router = feature_router();
    let oversized_payload: String = "x".repeat(128);
    let request: Request<Body> = Request::builder()
        .method(Method::POST)
        .uri("/echo")
        .header("content-type", "application/json")
        .body(Body::from(oversized_payload))
        .expect("request build should succeed");

    let response: axum::http::Response<Body> = app
        .oneshot(request)
        .await
        .expect("request should be handled");
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn global_error_fallback_returns_500_for_unknown_error() {
    let unknown_error: axum::BoxError = std::io::Error::other("unexpected").into();

    let response: axum::http::Response<Body> =
        handle_global_error(unknown_error).await.into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
