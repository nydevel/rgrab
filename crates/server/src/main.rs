mod config;

use anyhow::Result;
use axum::Router;
use storage::rocks_store::RocksStore;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Config::load();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cfg.log_level))
        .init();

    let store = RocksStore::open(&cfg.data_dir)?;
    tracing::info!("RocksDB opened at {}", cfg.data_dir);

    let app = Router::new()
        .merge(web::api::router(store.clone()))
        .merge(web::loki_api::router(store.clone()))
        .merge(collector::ingest::router(store))
        .layer(CorsLayer::permissive());

    tracing::info!("Server listening on {}", cfg.listen);
    let listener = tokio::net::TcpListener::bind(&cfg.listen).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
