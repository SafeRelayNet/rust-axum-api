use tracing_subscriber::EnvFilter;

/// Initializes tracing for the process at startup.
///
/// Keeping this in infrastructure preserves a thin `main.rs` and keeps
/// runtime wiring concerns grouped in the composition layer.
pub fn initialize_tracing() {
    let fallback_filter: EnvFilter = EnvFilter::new("my_axum_project=info,sqlx=warn,axum=warn");
    let env_filter: EnvFilter = EnvFilter::try_from_default_env().unwrap_or(fallback_filter);

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
        .init();
}
