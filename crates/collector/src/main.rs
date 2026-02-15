mod ingest;

use anyhow::Result;
use storage::rocks_store::RocksStore;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let data_dir = std::env::var("RGRAB_DATA_DIR").unwrap_or_else(|_| "./data/rgrab".to_string());
    let store = RocksStore::open(&data_dir)?;

    let app = ingest::router(store);

    let addr = "0.0.0.0:4317";
    tracing::info!("Collector listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
