use std::collections::HashMap;

use common::log::LogEntry;
use common::span::Span;

use crate::client::ApiClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Logs,
    Traces,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}

pub struct App {
    pub tab: Tab,
    pub logs: Vec<LogEntry>,
    pub traces: Vec<Span>,
    pub trace_spans: HashMap<String, Vec<Span>>,
    pub labels: Vec<String>,
    pub label_values: HashMap<String, Vec<String>>,
    pub selected_labels: HashMap<String, String>,
    pub search: String,
    pub input_mode: InputMode,
    pub log_scroll: usize,
    pub trace_scroll: usize,
    pub sidebar_scroll: usize,
    pub sidebar_focused: bool,
    pub expanded_trace: Option<String>,
    pub live_tail: bool,
    pub limit: usize,
    pub should_quit: bool,
    pub error: Option<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            tab: Tab::Logs,
            logs: Vec::new(),
            traces: Vec::new(),
            trace_spans: HashMap::new(),
            labels: Vec::new(),
            label_values: HashMap::new(),
            selected_labels: HashMap::new(),
            search: String::new(),
            input_mode: InputMode::Normal,
            log_scroll: 0,
            trace_scroll: 0,
            sidebar_scroll: 0,
            sidebar_focused: false,
            expanded_trace: None,
            live_tail: true,
            limit: 200,
            should_quit: false,
            error: None,
        }
    }

    pub async fn refresh(&mut self, client: &ApiClient) {
        self.error = None;

        match client.fetch_logs(self.limit).await {
            Ok(logs) => self.logs = logs,
            Err(e) => self.error = Some(format!("logs: {e}")),
        }

        match client.fetch_traces(self.limit).await {
            Ok(traces) => self.traces = traces,
            Err(e) => {
                if self.error.is_none() {
                    self.error = Some(format!("traces: {e}"));
                }
            }
        }

        match client.fetch_labels().await {
            Ok(labels) => {
                self.labels = labels;
                self.refresh_label_values(client).await;
            }
            Err(e) => {
                if self.error.is_none() {
                    self.error = Some(format!("labels: {e}"));
                }
            }
        }
    }

    async fn refresh_label_values(&mut self, client: &ApiClient) {
        for label in &self.labels {
            if !self.label_values.contains_key(label)
                && let Ok(values) = client.fetch_label_values(label).await
            {
                self.label_values.insert(label.clone(), values);
            }
        }
    }

    pub async fn expand_trace(&mut self, client: &ApiClient, trace_id: &str) {
        if !self.trace_spans.contains_key(trace_id)
            && let Ok(spans) = client.fetch_trace(trace_id).await
        {
            self.trace_spans.insert(trace_id.to_string(), spans);
        }
        self.expanded_trace = Some(trace_id.to_string());
    }

    pub fn filtered_logs(&self) -> Vec<&LogEntry> {
        self.logs
            .iter()
            .filter(|log| {
                if !self.search.is_empty() {
                    let search_lower = self.search.to_lowercase();
                    let msg_match = log.message.to_lowercase().contains(&search_lower);
                    if !msg_match {
                        return false;
                    }
                }
                for (key, val) in &self.selected_labels {
                    match log.labels.get(key) {
                        Some(v) if v == val => {}
                        _ => return false,
                    }
                }
                true
            })
            .collect()
    }

    pub fn unique_traces(&self) -> Vec<TraceGroup> {
        let mut groups: HashMap<String, TraceGroup> = HashMap::new();

        for span in &self.traces {
            let group = groups
                .entry(span.trace_id.clone())
                .or_insert_with(|| TraceGroup {
                    trace_id: span.trace_id.clone(),
                    root_operation: String::new(),
                    root_service: String::new(),
                    start_time: span.start_time,
                    total_duration_ms: 0.0,
                    span_count: 0,
                    has_error: false,
                });
            group.span_count += 1;
            if span.parent_span_id.is_none() {
                group.root_operation = span.operation_name.clone();
                group.root_service = span.service_name.clone();
                let dur = span
                    .end_time
                    .signed_duration_since(span.start_time)
                    .num_microseconds()
                    .unwrap_or(0) as f64
                    / 1000.0;
                group.total_duration_ms = dur;
                group.start_time = span.start_time;
            }
            if span.status == common::span::SpanStatus::Error {
                group.has_error = true;
            }
        }

        let mut result: Vec<TraceGroup> = groups.into_values().collect();
        result.sort_by(|a, b| b.start_time.cmp(&a.start_time));
        result
    }

    pub fn sidebar_items(&self) -> Vec<SidebarItem> {
        let mut items = Vec::new();
        for label in &self.labels {
            items.push(SidebarItem::Label(label.clone()));
            if let Some(values) = self.label_values.get(label) {
                for val in values {
                    let selected = self.selected_labels.get(label) == Some(val);
                    items.push(SidebarItem::Value {
                        label: label.clone(),
                        value: val.clone(),
                        selected,
                    });
                }
            }
        }
        items
    }

    pub fn toggle_label(&mut self, label: &str, value: &str) {
        if self.selected_labels.get(label) == Some(&value.to_string()) {
            self.selected_labels.remove(label);
        } else {
            self.selected_labels
                .insert(label.to_string(), value.to_string());
        }
    }
}

pub struct TraceGroup {
    pub trace_id: String,
    pub root_operation: String,
    pub root_service: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub total_duration_ms: f64,
    pub span_count: usize,
    pub has_error: bool,
}

pub enum SidebarItem {
    Label(String),
    Value {
        label: String,
        value: String,
        selected: bool,
    },
}
