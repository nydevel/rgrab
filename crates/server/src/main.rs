use anyhow::Result;
use axum::Router;
use axum::response::Redirect;
use axum::routing::get;
use storage::rocks_store::RocksStore;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let data_dir = std::env::var("RGRAB_DATA_DIR").unwrap_or_else(|_| "./data/rgrab".to_string());
    let store = RocksStore::open(&data_dir)?;
    tracing::info!("RocksDB opened at {data_dir}");

    let static_dir = std::env::var("RGRAB_STATIC_DIR").unwrap_or_else(|_| find_static_dir());

    let app = Router::new()
        .merge(web::api::router(store.clone()))
        .merge(web::loki_api::router(store.clone()))
        .merge(collector::ingest::router(store))
        .nest_service("/static", ServeDir::new(&static_dir))
        .route(
            "/",
            get(|| async { Redirect::permanent("/static/index.html") }),
        )
        .layer(CorsLayer::permissive());

    let addr = std::env::var("RGRAB_LISTEN").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    tracing::info!("Server listening on {addr}");
    tracing::info!("Static files from {static_dir}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn find_static_dir() -> String {
    let candidates = ["static", "crates/server/static", "../server/static"];
    for c in candidates {
        if std::path::Path::new(c).is_dir() {
            return c.to_string();
        }
    }
    "static".to_string()
}
