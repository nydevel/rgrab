use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use common::log::LogEntry;
use common::otlp::{ExportTraceServiceRequest, extract_spans};
use common::span::Span;
use storage::rocks_store::RocksStore;

pub fn router(store: RocksStore) -> Router {
    Router::new()
        .route("/v1/logs", post(ingest_logs))
        .route("/v1/traces", post(ingest_traces))
        .route("/otlp/v1/traces", post(otlp_ingest_traces))
        .with_state(store)
}

async fn ingest_logs(
    State(store): State<RocksStore>,
    Json(entries): Json<Vec<LogEntry>>,
) -> StatusCode {
    for entry in entries {
        if let Err(e) = store.insert_log(entry).await {
            tracing::error!("Failed to insert log: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }
    StatusCode::OK
}

async fn ingest_traces(
    State(store): State<RocksStore>,
    Json(spans): Json<Vec<Span>>,
) -> StatusCode {
    for span in spans {
        if let Err(e) = store.insert_span(span).await {
            tracing::error!("Failed to insert span: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }
    StatusCode::OK
}

async fn otlp_ingest_traces(
    State(store): State<RocksStore>,
    Json(req): Json<ExportTraceServiceRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let spans = extract_spans(&req);
    for span in spans {
        if let Err(e) = store.insert_span(span).await {
            tracing::error!("Failed to insert OTLP span: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({})),
            );
        }
    }
    (StatusCode::OK, Json(serde_json::json!({})))
}
