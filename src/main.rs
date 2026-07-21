use std::{net::SocketAddr, sync::Arc};

use agent_terrace::{api_router, AppState, ProcessRunner};
use tokio::net::TcpListener;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let address: SocketAddr = std::env::var("AGENT_TERRACE_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".into())
        .parse()?;
    let static_files =
        ServeDir::new("client/dist").not_found_service(ServeFile::new("client/dist/index.html"));
    let app = api_router(AppState::new(Arc::new(ProcessRunner)))
        .fallback_service(static_files)
        .layer(TraceLayer::new_for_http());
    let listener = TcpListener::bind(address).await?;

    info!(%address, "agent terrace is listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
