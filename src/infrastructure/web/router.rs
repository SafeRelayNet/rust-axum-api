use anyhow::Result;
use axum::error_handling::HandleErrorLayer;
use axum::extract::DefaultBodyLimit;
use axum::middleware::from_fn;
use axum::Router;
use listenfd::ListenFd;
use tokio::net::TcpListener;
use tokio::signal;
use tower::timeout::TimeoutLayer;
use tower::ServiceBuilder;

use crate::infrastructure::state::AppState;
use crate::infrastructure::web::auth;
use crate::infrastructure::web::debug;
use crate::infrastructure::web::error::handle_global_error;
use crate::infrastructure::web::response::response_wrapper;

pub fn create_app(state: AppState) -> Router {
    let timeout_seconds: u64 = state.environment.default_timeout_seconds;
    let max_request_body_size: usize = state.environment.max_request_body_size;

    Router::new()
        .merge(auth::routes())
        .merge(debug::routes())
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_global_error))
                .layer(TimeoutLayer::new(std::time::Duration::from_secs(
                    timeout_seconds,
                ))),
        )
        .layer(from_fn(response_wrapper))
        .layer(DefaultBodyLimit::max(max_request_body_size))
        .with_state(state)
}

pub async fn setup_listener(state: &AppState) -> Result<TcpListener> {
    let listener: TcpListener = match ListenFd::from_env().take_tcp_listener(0)? {
        Some(std_listener) => {
            std_listener.set_nonblocking(true)?;
            TcpListener::from_std(std_listener)?
        }
        None => {
            // Normalize localhost to IPv4 loopback so tools using 127.0.0.1
            // can connect consistently on macOS/Linux.
            let host: &str = if state.environment.host.as_ref() == "localhost" {
                "127.0.0.1"
            } else {
                state.environment.host.as_ref()
            };
            let address: String = format!("{host}:{}", state.environment.port);
            TcpListener::bind(&address).await?
        }
    };

    Ok(listener)
}

pub async fn shutdown_signal(state: AppState) {
    let ctrl_c_signal = async {
        let _result: Result<(), std::io::Error> = signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate_signal = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                let _value: Option<()> = stream.recv().await;
            }
            Err(_error) => {}
        }
    };

    #[cfg(not(unix))]
    let terminate_signal: std::future::Pending<()> = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c_signal => tracing::info!("Shutting down via Ctrl+C"),
        _ = terminate_signal => tracing::info!("Shutting down via TERM signal"),
    }

    state.shutdown().await;
}
