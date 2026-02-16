use std::collections::HashMap;

use bollard::Docker;
use bollard::container::LogsOptions;
use chrono::Utc;
use common::log::LogEntry;
use futures_util::StreamExt;
use storage::rocks_store::RocksStore;

use super::log_parser;

pub async fn stream_container(
    docker: Docker,
    container_id: String,
    container_name: String,
    labels: HashMap<String, String>,
    store: RocksStore,
    tail: usize,
) {
    let opts = LogsOptions::<String> {
        follow: true,
        stdout: true,
        stderr: true,
        timestamps: true,
        tail: tail.to_string(),
        ..Default::default()
    };

    let mut stream = docker.logs(&container_id, Some(opts));

    tracing::info!("Streaming logs from {container_name}");

    while let Some(result) = stream.next().await {
        let output = match result {
            Ok(o) => o,
            Err(e) => {
                tracing::debug!("Log stream error for {container_name}: {e}");
                break;
            }
        };

        let (raw, stream_type) = match &output {
            bollard::container::LogOutput::StdOut { message } => {
                (String::from_utf8_lossy(message).to_string(), "stdout")
            }
            bollard::container::LogOutput::StdErr { message } => {
                (String::from_utf8_lossy(message).to_string(), "stderr")
            }
            _ => continue,
        };

        let line = raw.trim_end();
        if line.is_empty() {
            continue;
        }

        let (timestamp, message) = parse_docker_timestamp(line);

        let message = message.trim();
        if message.is_empty() || chrono::DateTime::parse_from_rfc3339(message).is_ok() {
            continue;
        }

        let level = log_parser::parse_level(message);

        let mut entry_labels = labels.clone();
        entry_labels.insert("stream".to_string(), stream_type.to_string());

        let entry = LogEntry {
            timestamp,
            level,
            message: message.to_string(),
            labels: entry_labels,
            trace_id: None,
            span_id: None,
        };

        if let Err(e) = store.insert_log(entry).await {
            tracing::error!("Failed to insert docker log: {e}");
        }
    }

    tracing::info!("Stopped streaming logs from {container_name}");
}

fn parse_docker_timestamp(line: &str) -> (chrono::DateTime<Utc>, &str) {
    if let Some(space_idx) = line.find(' ') {
        let ts_str = &line[..space_idx];
        if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(ts_str) {
            return (ts.with_timezone(&Utc), &line[space_idx + 1..]);
        }
    }
    (Utc::now(), line)
}
