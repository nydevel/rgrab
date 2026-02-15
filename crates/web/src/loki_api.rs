use std::collections::{BTreeMap, HashMap};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use common::label_selector::parse_label_selector;
use common::log::{LogEntry, LogLevel};
use common::loki::{
    Direction, LokiLabelsResponse, LokiPushRequest, LokiQueryData, LokiQueryResponse,
    LokiResultStream,
};
use serde::Deserialize;
use storage::rocks_store::RocksStore;

pub fn router(store: RocksStore) -> Router {
    Router::new()
        .route("/rgrab/api/v1/push", post(loki_push))
        .route("/rgrab/api/v1/query", get(loki_query))
        .route("/rgrab/api/v1/query_range", get(loki_query_range))
        .route("/rgrab/api/v1/labels", get(loki_labels))
        .route("/rgrab/api/v1/label/{name}/values", get(loki_label_values))
        .with_state(store)
}

async fn loki_push(
    State(store): State<RocksStore>,
    Json(req): Json<LokiPushRequest>,
) -> StatusCode {
    for stream in req.streams {
        let labels = stream.stream;
        for parts in &stream.values {
            let ts_nanos: i64 = parts
                .first()
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let message = parts
                .get(1)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let (trace_id, span_id) = extract_metadata(parts.get(2));

            let level = labels
                .get("level")
                .and_then(|l| parse_log_level(l))
                .unwrap_or(LogLevel::Info);

            let entry = LogEntry {
                timestamp: chrono::DateTime::from_timestamp_nanos(ts_nanos),
                level,
                message,
                labels: labels.clone(),
                trace_id,
                span_id,
            };
            if let Err(e) = store.insert_log(entry).await {
                tracing::error!("Failed to insert log: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
    }
    StatusCode::NO_CONTENT
}

#[derive(Deserialize)]
struct LokiQueryParams {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    time: Option<String>,
    #[serde(default)]
    direction: Direction,
}

async fn loki_query(
    State(store): State<RocksStore>,
    Query(params): Query<LokiQueryParams>,
) -> Result<Json<LokiQueryResponse>, StatusCode> {
    let matchers = parse_label_selector(&params.query).map_err(|_| StatusCode::BAD_REQUEST)?;

    let end = params.time.as_deref().and_then(|t| t.parse::<i64>().ok());

    let logs = store
        .query_logs_filtered(matchers, None, end, params.limit, params.direction)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(build_streams_response(logs)))
}

#[derive(Deserialize)]
struct LokiQueryRangeParams {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    start: Option<String>,
    end: Option<String>,
    #[serde(default)]
    direction: Direction,
}

async fn loki_query_range(
    State(store): State<RocksStore>,
    Query(params): Query<LokiQueryRangeParams>,
) -> Result<Json<LokiQueryResponse>, StatusCode> {
    let matchers = parse_label_selector(&params.query).map_err(|_| StatusCode::BAD_REQUEST)?;

    let start = params.start.as_deref().and_then(|t| t.parse::<i64>().ok());
    let end = params.end.as_deref().and_then(|t| t.parse::<i64>().ok());

    let logs = store
        .query_logs_filtered(matchers, start, end, params.limit, params.direction)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(build_streams_response(logs)))
}

#[derive(Deserialize)]
struct LokiLabelsParams {
    start: Option<String>,
    end: Option<String>,
}

async fn loki_labels(
    State(store): State<RocksStore>,
    Query(params): Query<LokiLabelsParams>,
) -> Result<Json<LokiLabelsResponse>, StatusCode> {
    let start = params.start.as_deref().and_then(|t| t.parse::<i64>().ok());
    let end = params.end.as_deref().and_then(|t| t.parse::<i64>().ok());

    let names = store
        .label_names(start, end)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LokiLabelsResponse {
        status: "success".to_string(),
        data: names,
    }))
}

async fn loki_label_values(
    State(store): State<RocksStore>,
    Path(name): Path<String>,
    Query(params): Query<LokiLabelsParams>,
) -> Result<Json<LokiLabelsResponse>, StatusCode> {
    let start = params.start.as_deref().and_then(|t| t.parse::<i64>().ok());
    let end = params.end.as_deref().and_then(|t| t.parse::<i64>().ok());

    let values = store
        .label_values(name, start, end)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LokiLabelsResponse {
        status: "success".to_string(),
        data: values,
    }))
}

fn default_limit() -> usize {
    100
}

fn build_streams_response(logs: Vec<LogEntry>) -> LokiQueryResponse {
    let mut streams: HashMap<BTreeMap<String, String>, Vec<[String; 2]>> = HashMap::new();

    for log in &logs {
        let key: BTreeMap<_, _> = log.labels.clone().into_iter().collect();
        let ts_nanos = log.timestamp.timestamp_nanos_opt().unwrap_or(0);
        streams
            .entry(key)
            .or_default()
            .push([ts_nanos.to_string(), log.message.clone()]);
    }

    let result = streams
        .into_iter()
        .map(|(labels, values)| LokiResultStream {
            stream: labels.into_iter().collect(),
            values,
        })
        .collect();

    LokiQueryResponse {
        status: "success".to_string(),
        data: LokiQueryData {
            result_type: "streams".to_string(),
            result,
            stats: serde_json::json!({}),
        },
    }
}

fn extract_metadata(meta: Option<&serde_json::Value>) -> (Option<String>, Option<String>) {
    let Some(obj) = meta.and_then(|v| v.as_object()) else {
        return (None, None);
    };
    let trace_id = obj
        .get("trace_id")
        .and_then(|v| v.as_str())
        .map(String::from);
    let span_id = obj
        .get("span_id")
        .and_then(|v| v.as_str())
        .map(String::from);
    (trace_id, span_id)
}

fn parse_log_level(s: &str) -> Option<LogLevel> {
    match s.to_lowercase().as_str() {
        "trace" => Some(LogLevel::Trace),
        "debug" => Some(LogLevel::Debug),
        "info" => Some(LogLevel::Info),
        "warn" | "warning" => Some(LogLevel::Warn),
        "error" | "err" => Some(LogLevel::Error),
        "fatal" | "critical" => Some(LogLevel::Fatal),
        _ => None,
    }
}
