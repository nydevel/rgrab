use std::collections::HashMap;

use chrono::Utc;
use common::log::{LogEntry, LogLevel};

#[test]
fn should_serialize_log_entry_to_json() {
    let entry = LogEntry {
        timestamp: chrono::DateTime::from_timestamp_nanos(1_000_000_000),
        level: LogLevel::Info,
        message: "hello".to_string(),
        labels: HashMap::from([("service".to_string(), "web".to_string())]),
        trace_id: None,
        span_id: None,
    };

    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("\"message\":\"hello\""));
    assert!(json.contains("\"level\":\"INFO\""));
}

#[test]
fn should_deserialize_log_entry_from_json() {
    let json = r#"{
        "timestamp": "2025-01-01T00:00:00Z",
        "level": "ERROR",
        "message": "something failed",
        "labels": {"service": "api"},
        "trace_id": null,
        "span_id": null
    }"#;

    let entry: LogEntry = serde_json::from_str(json).unwrap();
    assert_eq!(entry.level, LogLevel::Error);
    assert_eq!(entry.message, "something failed");
    assert_eq!(entry.labels.get("service").unwrap(), "api");
}

#[test]
fn should_roundtrip_all_log_levels() {
    let levels = [
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
        LogLevel::Fatal,
    ];

    for level in levels {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level,
            message: "test".to_string(),
            labels: HashMap::new(),
            trace_id: None,
            span_id: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: LogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.level, level);
    }
}

#[test]
fn should_serialize_log_with_trace_ids() {
    let entry = LogEntry {
        timestamp: Utc::now(),
        level: LogLevel::Info,
        message: "traced".to_string(),
        labels: HashMap::new(),
        trace_id: Some("abc123".to_string()),
        span_id: Some("def456".to_string()),
    };

    let json = serde_json::to_string(&entry).unwrap();
    let parsed: LogEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.trace_id.as_deref(), Some("abc123"));
    assert_eq!(parsed.span_id.as_deref(), Some("def456"));
}
