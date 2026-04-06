use std::convert::Infallible;

use axum::body::Body;
use axum::http::header::{HeaderValue, CONTENT_TYPE};
use axum::http::{Extensions, Request, Response, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize)]
pub struct ResponseFormat {
    pub status: String,
    pub code: u16,
    pub data: serde_json::Value,
    pub messages: Vec<String>,
    pub date: String,
}

#[derive(Debug, Clone)]
pub struct HandlerResponse {
    pub status_code: StatusCode,
    pub data: serde_json::Value,
    pub messages: Vec<String>,
}

impl HandlerResponse {
    pub fn new(status_code: StatusCode) -> Self {
        Self {
            status_code,
            data: serde_json::Value::Null,
            messages: Vec::new(),
        }
    }

    pub fn data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.messages.push(message.into());
        self
    }
}

impl IntoResponse for HandlerResponse {
    fn into_response(self) -> axum::response::Response {
        let mut response: Response<Body> = Json(json!({
            "data": self.data,
            "messages": self.messages
        }))
        .into_response();

        *response.status_mut() = self.status_code;
        response.extensions_mut().insert(self);
        response
    }
}

fn extract_response_components(response: &Response<Body>) -> (Vec<String>, Value) {
    let extensions: &Extensions = response.extensions();
    let structured_response: Option<&HandlerResponse> = extensions.get::<HandlerResponse>();

    match structured_response {
        Some(payload) => (payload.messages.clone(), payload.data.clone()),
        None => (Vec::new(), Value::Null),
    }
}

pub async fn response_wrapper(
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, Infallible> {
    let response: Response<Body> = next.run(request).await;
    let (messages, data): (Vec<String>, Value) = extract_response_components(&response);
    let (mut parts, _body): (axum::http::response::Parts, Body) = response.into_parts();

    let status_name: String = parts
        .status
        .canonical_reason()
        .unwrap_or("UNKNOWN_STATUS")
        .to_uppercase()
        .replace(' ', "_");
    let wrapped: ResponseFormat = ResponseFormat {
        status: status_name,
        code: parts.status.as_u16(),
        data,
        messages,
        date: Utc::now().to_rfc3339(),
    };

    let response_body: Vec<u8> = serde_json::to_vec(&wrapped).unwrap_or_else(|_| b"{}".to_vec());
    parts
        .headers
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    Ok(Response::from_parts(parts, Body::from(response_body)))
}
