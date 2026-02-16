mod config;

use anyhow::Result;
use axum::Router;
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use storage::rocks_store::RocksStore;
use tower::Service;
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

    loop {
        let (stream, _addr) = listener.accept().await?;
        let tower_service = app.clone();

        tokio::spawn(async move {
            let hyper_service = hyper::service::service_fn(move |req: hyper::Request<Incoming>| {
                let mut svc = tower_service.clone();
                async move { svc.call(req).await }
            });

            let builder = Builder::new(TokioExecutor::new());
            if let Err(e) = builder
                .serve_connection(TokioIo::new(stream), hyper_service)
                .await
            {
                tracing::debug!("Connection error: {}", e);
            }
        });
    }
}
