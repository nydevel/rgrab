use anyhow::Result;
use axum::Router;
use storage::rocks_store::RocksStore;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let data_dir = std::env::var("RGRAB_DATA_DIR").unwrap_or_else(|_| "./data/rgrab".to_string());
    let store = RocksStore::open(&data_dir)?;
    tracing::info!("RocksDB opened at {data_dir}");

    let app = Router::new()
        .merge(web::api::router(store.clone()))
        .merge(web::loki_api::router(store.clone()))
        .merge(collector::ingest::router(store))
        .layer(CorsLayer::permissive());

    let addr = std::env::var("RGRAB_LISTEN").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    tracing::info!("Server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
