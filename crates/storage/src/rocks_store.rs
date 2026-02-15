use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use common::log::LogEntry;
use common::loki::{Direction, LabelMatcher};
use common::span::Span;
use rocksdb::{ColumnFamilyDescriptor, DB, IteratorMode, Options};

const CF_LOGS: &str = "logs";
const CF_SPANS: &str = "spans";

#[derive(Clone)]
pub struct RocksStore {
    db: Arc<DB>,
    log_seq: Arc<AtomicU64>,
}

impl RocksStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_logs = ColumnFamilyDescriptor::new(CF_LOGS, Options::default());
        let cf_spans = ColumnFamilyDescriptor::new(CF_SPANS, Options::default());

        let db = DB::open_cf_descriptors(&opts, path, vec![cf_logs, cf_spans])
            .context("Failed to open RocksDB")?;

        let log_seq = recover_sequence(&db)?;

        Ok(Self {
            db: Arc::new(db),
            log_seq: Arc::new(AtomicU64::new(log_seq)),
        })
    }

    pub async fn insert_log(&self, entry: LogEntry) -> Result<()> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.insert_log_sync(&entry)).await?
    }

    pub async fn insert_span(&self, span: Span) -> Result<()> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.insert_span_sync(&span)).await?
    }

    pub async fn query_logs(&self, limit: usize) -> Result<Vec<LogEntry>> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.query_logs_sync(limit)).await?
    }

    pub async fn query_spans(&self, trace_id: String) -> Result<Vec<Span>> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.query_spans_sync(&trace_id)).await?
    }

    pub async fn query_all_spans(&self, limit: usize) -> Result<Vec<Span>> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.query_all_spans_sync(limit)).await?
    }

    pub async fn query_logs_filtered(
        &self,
        matchers: Vec<LabelMatcher>,
        start: Option<i64>,
        end: Option<i64>,
        limit: usize,
        direction: Direction,
    ) -> Result<Vec<LogEntry>> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            this.query_logs_filtered_sync(&matchers, start, end, limit, direction)
        })
        .await?
    }

    pub async fn label_names(&self, start: Option<i64>, end: Option<i64>) -> Result<Vec<String>> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.label_names_sync(start, end)).await?
    }

    pub async fn label_values(
        &self,
        label_name: String,
        start: Option<i64>,
        end: Option<i64>,
    ) -> Result<Vec<String>> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.label_values_sync(&label_name, start, end)).await?
    }
}

impl RocksStore {
    fn insert_log_sync(&self, entry: &LogEntry) -> Result<()> {
        let cf = self.db.cf_handle(CF_LOGS).context("logs CF missing")?;
        let ts_nanos = entry
            .timestamp
            .timestamp_nanos_opt()
            .context("timestamp out of range")?;
        let seq = self.log_seq.fetch_add(1, Ordering::Relaxed);
        let key = encode_log_key(ts_nanos, seq);
        let value = serde_json::to_vec(entry)?;
        self.db.put_cf(&cf, key, value)?;
        Ok(())
    }

    fn insert_span_sync(&self, span: &Span) -> Result<()> {
        let cf = self.db.cf_handle(CF_SPANS).context("spans CF missing")?;
        let key = encode_span_key(&span.trace_id, &span.span_id);
        let value = serde_json::to_vec(span)?;
        self.db.put_cf(&cf, key, value)?;
        Ok(())
    }

    fn query_logs_sync(&self, limit: usize) -> Result<Vec<LogEntry>> {
        let cf = self.db.cf_handle(CF_LOGS).context("logs CF missing")?;
        let iter = self.db.iterator_cf(&cf, IteratorMode::End);
        let mut results = Vec::with_capacity(limit);
        for item in iter.take(limit) {
            let (_, value) = item?;
            results.push(serde_json::from_slice(&value)?);
        }
        Ok(results)
    }

    fn query_spans_sync(&self, trace_id: &str) -> Result<Vec<Span>> {
        let cf = self.db.cf_handle(CF_SPANS).context("spans CF missing")?;
        let prefix = span_key_prefix(trace_id);
        let iter = self.db.iterator_cf(
            &cf,
            IteratorMode::From(&prefix, rocksdb::Direction::Forward),
        );
        let mut results = Vec::new();
        for item in iter {
            let (key, value) = item?;
            if !key.starts_with(&prefix) {
                break;
            }
            results.push(serde_json::from_slice(&value)?);
        }
        Ok(results)
    }

    fn query_all_spans_sync(&self, limit: usize) -> Result<Vec<Span>> {
        let cf = self.db.cf_handle(CF_SPANS).context("spans CF missing")?;
        let iter = self.db.iterator_cf(&cf, IteratorMode::End);
        let mut results = Vec::with_capacity(limit);
        for item in iter.take(limit) {
            let (_, value) = item?;
            results.push(serde_json::from_slice(&value)?);
        }
        Ok(results)
    }

    fn query_logs_filtered_sync(
        &self,
        matchers: &[LabelMatcher],
        start: Option<i64>,
        end: Option<i64>,
        limit: usize,
        direction: Direction,
    ) -> Result<Vec<LogEntry>> {
        let cf = self.db.cf_handle(CF_LOGS).context("logs CF missing")?;
        let mut raw_iter = self.db.raw_iterator_cf(&cf);

        match direction {
            Direction::Backward => {
                if let Some(end_ns) = end {
                    let key = encode_log_key(end_ns, u64::MAX);
                    raw_iter.seek_for_prev(key);
                } else {
                    raw_iter.seek_to_last();
                }
            }
            Direction::Forward => {
                if let Some(start_ns) = start {
                    let key = encode_log_key(start_ns, 0);
                    raw_iter.seek(key);
                } else {
                    raw_iter.seek_to_first();
                }
            }
        }

        let mut results = Vec::with_capacity(limit);
        while raw_iter.valid() && results.len() < limit {
            let Some(key) = raw_iter.key() else { break };
            let Some(value) = raw_iter.value() else {
                break;
            };

            if key.len() == 16 {
                let ts = i64::from_be_bytes(key[..8].try_into().unwrap());
                if let Some(s) = start
                    && ts < s
                {
                    match direction {
                        Direction::Forward => {
                            raw_iter.next();
                            continue;
                        }
                        Direction::Backward => break,
                    }
                }
                if let Some(e) = end
                    && ts > e
                {
                    match direction {
                        Direction::Backward => {
                            raw_iter.prev();
                            continue;
                        }
                        Direction::Forward => break,
                    }
                }
            }

            let entry: LogEntry = serde_json::from_slice(value)?;
            if matchers.iter().all(|m| m.matches(&entry.labels)) {
                results.push(entry);
            }

            match direction {
                Direction::Forward => raw_iter.next(),
                Direction::Backward => raw_iter.prev(),
            }
        }
        raw_iter.status()?;
        Ok(results)
    }

    fn label_names_sync(&self, start: Option<i64>, end: Option<i64>) -> Result<Vec<String>> {
        let cf = self.db.cf_handle(CF_LOGS).context("logs CF missing")?;
        let mut names = BTreeSet::new();

        for_each_log_in_range(&self.db, &cf, start, end, |entry| {
            names.extend(entry.labels.keys().cloned());
        })?;

        names.insert("level".to_string());
        Ok(names.into_iter().collect())
    }

    fn label_values_sync(
        &self,
        label_name: &str,
        start: Option<i64>,
        end: Option<i64>,
    ) -> Result<Vec<String>> {
        let cf = self.db.cf_handle(CF_LOGS).context("logs CF missing")?;
        let mut values = BTreeSet::new();

        for_each_log_in_range(&self.db, &cf, start, end, |entry| {
            if label_name == "level" {
                values.insert(format!("{:?}", entry.level).to_lowercase());
            } else if let Some(v) = entry.labels.get(label_name) {
                values.insert(v.clone());
            }
        })?;

        Ok(values.into_iter().collect())
    }
}

fn recover_sequence(db: &DB) -> Result<u64> {
    let cf = db.cf_handle(CF_LOGS).context("logs CF missing")?;
    let mut iter = db.raw_iterator_cf(&cf);
    iter.seek_to_last();
    if iter.valid()
        && let Some(key) = iter.key()
        && key.len() == 16
    {
        let seq_bytes: [u8; 8] = key[8..16].try_into().unwrap();
        return Ok(u64::from_be_bytes(seq_bytes) + 1);
    }
    Ok(0)
}

fn encode_log_key(timestamp_nanos: i64, seq: u64) -> [u8; 16] {
    let mut key = [0u8; 16];
    key[..8].copy_from_slice(&timestamp_nanos.to_be_bytes());
    key[8..].copy_from_slice(&seq.to_be_bytes());
    key
}

fn encode_span_key(trace_id: &str, span_id: &str) -> Vec<u8> {
    format!("{trace_id}:{span_id}").into_bytes()
}

fn span_key_prefix(trace_id: &str) -> Vec<u8> {
    format!("{trace_id}:").into_bytes()
}

fn for_each_log_in_range(
    db: &DB,
    cf: &impl rocksdb::AsColumnFamilyRef,
    start: Option<i64>,
    end: Option<i64>,
    mut callback: impl FnMut(&LogEntry),
) -> Result<()> {
    let mut raw_iter = db.raw_iterator_cf(cf);

    if let Some(start_ns) = start {
        let key = encode_log_key(start_ns, 0);
        raw_iter.seek(key);
    } else {
        raw_iter.seek_to_first();
    }

    while raw_iter.valid() {
        let Some(key) = raw_iter.key() else { break };
        let Some(value) = raw_iter.value() else {
            break;
        };

        if key.len() == 16
            && let Some(e) = end
        {
            let ts = i64::from_be_bytes(key[..8].try_into().unwrap());
            if ts > e {
                break;
            }
        }

        let entry: LogEntry = serde_json::from_slice(value)?;
        callback(&entry);
        raw_iter.next();
    }
    raw_iter.status()?;
    Ok(())
}
