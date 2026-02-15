mod api;

use anyhow::Result;
use storage::store::InMemoryStore;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let store = InMemoryStore::new();
    let app = api::router(store).layer(CorsLayer::permissive());

    let addr = "0.0.0.0:3000";
    tracing::info!("Web UI listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
