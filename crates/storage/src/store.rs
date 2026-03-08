use std::sync::Arc;

use anyhow::Result;
use common::log::LogEntry;
use common::span::Span;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default)]
pub struct InMemoryStore {
    logs: Arc<RwLock<Vec<LogEntry>>>,
    spans: Arc<RwLock<Vec<Span>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn insert_log(&self, entry: LogEntry) -> Result<()> {
        self.logs.write().await.push(entry);
        Ok(())
    }

    pub async fn insert_span(&self, span: Span) -> Result<()> {
        self.spans.write().await.push(span);
        Ok(())
    }

    pub async fn query_logs(&self, limit: usize, offset: usize) -> Result<Vec<LogEntry>> {
        let logs = self.logs.read().await;
        let result: Vec<_> = logs
            .iter()
            .rev()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();
        Ok(result)
    }

    pub async fn query_spans(&self, trace_id: &str) -> Result<Vec<Span>> {
        let spans = self.spans.read().await;
        let result: Vec<_> = spans
            .iter()
            .filter(|s| s.trace_id == trace_id)
            .cloned()
            .collect();
        Ok(result)
    }

    pub async fn query_all_spans(&self, limit: usize) -> Result<Vec<Span>> {
        let spans = self.spans.read().await;
        let result: Vec<_> = spans.iter().rev().take(limit).cloned().collect();
        Ok(result)
    }
}
