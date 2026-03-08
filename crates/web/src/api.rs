use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use common::log::LogEntry;
use common::span::Span;
use serde::Deserialize;
use storage::rocks_store::RocksStore;

pub fn router(store: RocksStore) -> Router {
    Router::new()
        .route("/api/logs", get(get_logs))
        .route("/api/traces", get(get_traces))
        .with_state(store)
}

#[derive(Deserialize)]
struct LogsQuery {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

#[derive(Deserialize)]
struct TracesQuery {
    trace_id: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    100
}

async fn get_logs(
    State(store): State<RocksStore>,
    Query(q): Query<LogsQuery>,
) -> Json<Vec<LogEntry>> {
    let logs = match store.query_logs(q.limit, q.offset).await {
        Ok(logs) => logs,
        Err(e) => {
            tracing::error!("Failed to query logs: {e}");
            Vec::new()
        }
    };
    Json(logs)
}

async fn get_traces(
    State(store): State<RocksStore>,
    Query(q): Query<TracesQuery>,
) -> Json<Vec<Span>> {
    let spans = match q.trace_id {
        Some(id) => match store.query_spans(id).await {
            Ok(spans) => spans,
            Err(e) => {
                tracing::error!("Failed to query spans: {e}");
                Vec::new()
            }
        },
        None => match store.query_all_spans(q.limit).await {
            Ok(spans) => spans,
            Err(e) => {
                tracing::error!("Failed to query all spans: {e}");
                Vec::new()
            }
        },
    };
    Json(spans)
}
