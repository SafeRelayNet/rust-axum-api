use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::routing::post;
use axum::Router;
use my_axum_project::infrastructure::web::auth::{LoginRequest, LogoutRequest, RegisterRequest};
use my_axum_project::infrastructure::web::response::HandlerResponse;
use my_axum_project::infrastructure::web::validated_json::ValidatedJson;
use serde_json::json;
use tower::ServiceExt;

async fn register_validation_handler(
    ValidatedJson(_payload): ValidatedJson<RegisterRequest>,
) -> HandlerResponse {
    HandlerResponse::new(StatusCode::OK).message("validated")
}

async fn login_validation_handler(
    ValidatedJson(_payload): ValidatedJson<LoginRequest>,
) -> HandlerResponse {
    HandlerResponse::new(StatusCode::OK).message("validated")
}

async fn logout_validation_handler(
    ValidatedJson(_payload): ValidatedJson<LogoutRequest>,
) -> HandlerResponse {
    HandlerResponse::new(StatusCode::OK).message("validated")
}

#[tokio::test]
async fn register_rejects_invalid_email_format_payload() {
    let app: Router = Router::new().route("/auth/register", post(register_validation_handler));

    let request: Request<Body> = Request::builder()
        .method(Method::POST)
        .uri("/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": "not-an-email",
                "password": "supersecret"
            })
            .to_string(),
        ))
        .expect("request build should succeed");

    let response: axum::http::Response<Body> = app
        .oneshot(request)
        .await
        .expect("request should be handled");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn register_rejects_malformed_json_payload() {
    let app: Router = Router::new().route("/auth/register", post(register_validation_handler));

    let request: Request<Body> = Request::builder()
        .method(Method::POST)
        .uri("/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            "{\"email\":\"user@example.com\",\"password\":\"x\"",
        ))
        .expect("request build should succeed");

    let response: axum::http::Response<Body> = app
        .oneshot(request)
        .await
        .expect("request should be handled");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn login_rejects_empty_password_payload() {
    let app: Router = Router::new().route("/auth/login", post(login_validation_handler));

    let request: Request<Body> = Request::builder()
        .method(Method::POST)
        .uri("/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": "user@example.com",
                "password": ""
            })
            .to_string(),
        ))
        .expect("request build should succeed");

    let response: axum::http::Response<Body> = app
        .oneshot(request)
        .await
        .expect("request should be handled");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn logout_rejects_empty_token_payload() {
    let app: Router = Router::new().route("/auth/logout", post(logout_validation_handler));

    let request: Request<Body> = Request::builder()
        .method(Method::POST)
        .uri("/auth/logout")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "token": "" }).to_string()))
        .expect("request build should succeed");

    let response: axum::http::Response<Body> = app
        .oneshot(request)
        .await
        .expect("request should be handled");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn register_accepts_valid_payload() {
    let app: Router = Router::new().route("/auth/register", post(register_validation_handler));

    let request: Request<Body> = Request::builder()
        .method(Method::POST)
        .uri("/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": "user@example.com",
                "password": "supersecret"
            })
            .to_string(),
        ))
        .expect("request build should succeed");

    let response: axum::http::Response<Body> = app
        .oneshot(request)
        .await
        .expect("request should be handled");

    assert_eq!(response.status(), StatusCode::OK);
}
