use axum::serve;

use my_axum_project::infrastructure::logging::initialize_tracing;
use my_axum_project::infrastructure::state::AppState;
use my_axum_project::infrastructure::web::router;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_tracing();

    let state: AppState = AppState::build().await?;
    let app: axum::Router = router::create_app(state.clone());
    let listener: tokio::net::TcpListener = router::setup_listener(&state).await?;

    tracing::info!("Server listening on {}", listener.local_addr()?);

    serve(listener, app)
        .with_graceful_shutdown(router::shutdown_signal(state))
        .await?;

    Ok(())
}
