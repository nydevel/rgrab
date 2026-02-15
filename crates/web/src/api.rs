use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use common::log::LogEntry;
use common::span::Span;
use serde::Deserialize;
use storage::store::InMemoryStore;

pub fn router(store: InMemoryStore) -> Router {
    Router::new()
        .route("/api/logs", get(get_logs))
        .route("/api/traces", get(get_traces))
        .with_state(store)
}

#[derive(Deserialize)]
struct LogsQuery {
    #[serde(default = "default_limit")]
    limit: usize,
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
    State(store): State<InMemoryStore>,
    Query(q): Query<LogsQuery>,
) -> Json<Vec<LogEntry>> {
    let logs = store.query_logs(q.limit).await.unwrap_or_default();
    Json(logs)
}

async fn get_traces(
    State(store): State<InMemoryStore>,
    Query(q): Query<TracesQuery>,
) -> Json<Vec<Span>> {
    let spans = match q.trace_id {
        Some(ref id) => store.query_spans(id).await.unwrap_or_default(),
        None => store.query_all_spans(q.limit).await.unwrap_or_default(),
    };
    Json(spans)
}
