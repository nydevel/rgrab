use std::collections::HashMap;

use chrono::Utc;
use common::log::{LogEntry, LogLevel};
use common::loki::{Direction, LabelMatcher, MatchOp};
use common::span::{Span, SpanStatus};

fn make_test_log(message: &str, level: LogLevel) -> LogEntry {
    LogEntry {
        timestamp: Utc::now(),
        level,
        message: message.to_string(),
        labels: HashMap::from([("service".to_string(), "test".to_string())]),
        trace_id: None,
        span_id: None,
    }
}

fn make_test_span(trace_id: &str, span_id: &str, operation: &str) -> Span {
    Span {
        trace_id: trace_id.to_string(),
        span_id: span_id.to_string(),
        parent_span_id: None,
        operation_name: operation.to_string(),
        service_name: "test-svc".to_string(),
        start_time: Utc::now(),
        end_time: Utc::now(),
        status: SpanStatus::Ok,
        attributes: HashMap::new(),
        events: vec![],
    }
}

fn temp_db_path() -> String {
    let id = uuid::Uuid::new_v4();
    format!("/tmp/rgrab-test-{id}")
}

#[tokio::test]
async fn should_insert_and_query_logs() {
    let path = temp_db_path();
    let store = storage::rocks_store::RocksStore::open(&path).unwrap();

    store
        .insert_log(make_test_log("hello world", LogLevel::Info))
        .await
        .unwrap();
    store
        .insert_log(make_test_log("error occurred", LogLevel::Error))
        .await
        .unwrap();

    let logs = store.query_logs(10, 0).await.unwrap();
    assert_eq!(logs.len(), 2);

    std::fs::remove_dir_all(&path).ok();
}

#[tokio::test]
async fn should_respect_query_limit() {
    let path = temp_db_path();
    let store = storage::rocks_store::RocksStore::open(&path).unwrap();

    for i in 0..5 {
        store
            .insert_log(make_test_log(&format!("log {i}"), LogLevel::Info))
            .await
            .unwrap();
    }

    let logs = store.query_logs(3, 0).await.unwrap();
    assert_eq!(logs.len(), 3);

    std::fs::remove_dir_all(&path).ok();
}

#[tokio::test]
async fn should_insert_and_query_spans_by_trace_id() {
    let path = temp_db_path();
    let store = storage::rocks_store::RocksStore::open(&path).unwrap();

    store
        .insert_span(make_test_span("trace-a", "span-1", "GET /"))
        .await
        .unwrap();
    store
        .insert_span(make_test_span("trace-a", "span-2", "DB query"))
        .await
        .unwrap();
    store
        .insert_span(make_test_span("trace-b", "span-3", "POST /"))
        .await
        .unwrap();

    let spans_a = store.query_spans("trace-a".to_string()).await.unwrap();
    assert_eq!(spans_a.len(), 2);

    let spans_b = store.query_spans("trace-b".to_string()).await.unwrap();
    assert_eq!(spans_b.len(), 1);

    std::fs::remove_dir_all(&path).ok();
}

#[tokio::test]
async fn should_query_all_spans_with_limit() {
    let path = temp_db_path();
    let store = storage::rocks_store::RocksStore::open(&path).unwrap();

    for i in 0..5 {
        store
            .insert_span(make_test_span("trace-x", &format!("span-{i}"), "op"))
            .await
            .unwrap();
    }

    let spans = store.query_all_spans(3).await.unwrap();
    assert_eq!(spans.len(), 3);

    std::fs::remove_dir_all(&path).ok();
}

#[tokio::test]
async fn should_filter_logs_by_label_eq() {
    let path = temp_db_path();
    let store = storage::rocks_store::RocksStore::open(&path).unwrap();

    let mut web_log = make_test_log("web request", LogLevel::Info);
    web_log
        .labels
        .insert("service".to_string(), "web".to_string());
    store.insert_log(web_log).await.unwrap();

    let mut api_log = make_test_log("api request", LogLevel::Info);
    api_log
        .labels
        .insert("service".to_string(), "api".to_string());
    store.insert_log(api_log).await.unwrap();

    let matchers = vec![LabelMatcher {
        name: "service".to_string(),
        op: MatchOp::Eq,
        value: "web".to_string(),
    }];

    let logs = store
        .query_logs_filtered(matchers, None, None, 100, Direction::Backward)
        .await
        .unwrap();

    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].message, "web request");

    std::fs::remove_dir_all(&path).ok();
}

#[tokio::test]
async fn should_query_label_names() {
    let path = temp_db_path();
    let store = storage::rocks_store::RocksStore::open(&path).unwrap();

    let mut log = make_test_log("test", LogLevel::Info);
    log.labels.insert("env".to_string(), "prod".to_string());
    store.insert_log(log).await.unwrap();

    let names = store.label_names(None, None).await.unwrap();
    assert!(names.contains(&"service".to_string()));
    assert!(names.contains(&"env".to_string()));
    assert!(names.contains(&"level".to_string()));

    std::fs::remove_dir_all(&path).ok();
}

#[tokio::test]
async fn should_query_label_values() {
    let path = temp_db_path();
    let store = storage::rocks_store::RocksStore::open(&path).unwrap();

    let mut log1 = make_test_log("test1", LogLevel::Info);
    log1.labels.insert("service".to_string(), "web".to_string());
    store.insert_log(log1).await.unwrap();

    let mut log2 = make_test_log("test2", LogLevel::Error);
    log2.labels.insert("service".to_string(), "api".to_string());
    store.insert_log(log2).await.unwrap();

    let values = store
        .label_values("service".to_string(), None, None)
        .await
        .unwrap();
    assert!(values.contains(&"web".to_string()));
    assert!(values.contains(&"api".to_string()));

    std::fs::remove_dir_all(&path).ok();
}

#[tokio::test]
async fn should_return_empty_for_nonexistent_trace() {
    let path = temp_db_path();
    let store = storage::rocks_store::RocksStore::open(&path).unwrap();

    let spans = store.query_spans("nonexistent".to_string()).await.unwrap();
    assert!(spans.is_empty());

    std::fs::remove_dir_all(&path).ok();
}

#[tokio::test]
async fn should_persist_sequence_across_reopens() {
    let path = temp_db_path();

    {
        let store = storage::rocks_store::RocksStore::open(&path).unwrap();
        store
            .insert_log(make_test_log("first", LogLevel::Info))
            .await
            .unwrap();
    }

    {
        let store = storage::rocks_store::RocksStore::open(&path).unwrap();
        store
            .insert_log(make_test_log("second", LogLevel::Info))
            .await
            .unwrap();

        let logs = store.query_logs(10, 0).await.unwrap();
        assert_eq!(logs.len(), 2);
    }

    std::fs::remove_dir_all(&path).ok();
}
