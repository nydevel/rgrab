use std::collections::HashMap;

use common::log::{LogEntry, LogLevel};
use common::span::Span;
use ratatui::layout::Rect;

use crate::client::ApiClient;

#[derive(Default)]
pub struct LayoutAreas {
    pub header: Rect,
    pub level_tabs: Rect,
    pub sidebar: Rect,
    pub log_list: Rect,
    pub traces: Rect,
}

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

pub const ALL_LEVELS: [LogLevel; 6] = [
    LogLevel::Trace,
    LogLevel::Debug,
    LogLevel::Info,
    LogLevel::Warn,
    LogLevel::Error,
    LogLevel::Fatal,
];

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
    pub log_cursor: usize,
    pub trace_scroll: usize,
    pub sidebar_scroll: usize,
    pub sidebar_focused: bool,
    pub expanded_trace: Option<String>,
    pub live_tail: bool,
    pub limit: usize,
    pub should_quit: bool,
    pub error: Option<String>,
    pub connected: bool,
    pub server_url: String,
    pub level_enabled: [bool; 6],
    pub selected_log_idx: Option<usize>,
    pub newest_first: bool,
    pub areas: LayoutAreas,
}

impl App {
    pub fn new(server_url: &str) -> Self {
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
            log_cursor: 0,
            trace_scroll: 0,
            sidebar_scroll: 0,
            sidebar_focused: false,
            expanded_trace: None,
            live_tail: true,
            limit: 200,
            should_quit: false,
            error: None,
            connected: false,
            server_url: server_url.to_string(),
            level_enabled: [true; 6],
            selected_log_idx: None,
            newest_first: true,
            areas: LayoutAreas::default(),
        }
    }

    pub fn toggle_level(&mut self, idx: usize) {
        if idx < self.level_enabled.len() {
            self.level_enabled[idx] = !self.level_enabled[idx];
            self.log_cursor = 0;
        }
    }

    pub fn is_level_enabled(&self, level: LogLevel) -> bool {
        self.level_enabled[level_index(level)]
    }

    pub async fn refresh(&mut self, client: &ApiClient) {
        self.error = None;

        match client.fetch_logs(self.limit).await {
            Ok(logs) => {
                self.connected = true;
                self.logs = logs;
            }
            Err(e) => {
                self.connected = false;
                self.error = Some(format!("{e}"));
                return;
            }
        }

        match client.fetch_traces(self.limit).await {
            Ok(traces) => self.traces = traces,
            Err(e) => self.error = Some(format!("traces: {e}")),
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
        const SIDEBAR_LABELS: &[&str] = &["service", "environment"];

        for label in SIDEBAR_LABELS {
            if let Ok(values) = client.fetch_label_values(label).await {
                self.label_values.insert((*label).to_string(), values);
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
        let mut result: Vec<&LogEntry> = self
            .logs
            .iter()
            .filter(|log| self.log_matches_filters(log))
            .collect();

        if self.newest_first {
            result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        } else {
            result.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        }
        result
    }

    fn log_matches_filters(&self, log: &LogEntry) -> bool {
        if !self.is_level_enabled(log.level) {
            return false;
        }
        if !self.search.is_empty() && !self.log_matches_search(log) {
            return false;
        }
        self.selected_labels
            .iter()
            .all(|(key, val)| log.labels.get(key) == Some(val))
    }

    fn log_matches_search(&self, log: &LogEntry) -> bool {
        let search_lower = self.search.to_lowercase();
        let in_msg = log.message.to_lowercase().contains(&search_lower);
        let in_labels = log.labels.iter().any(|(k, v)| {
            k.to_lowercase().contains(&search_lower) || v.to_lowercase().contains(&search_lower)
        });
        in_msg || in_labels
    }

    pub fn unique_traces(&self) -> Vec<TraceGroup> {
        let mut groups: HashMap<String, TraceGroup> = HashMap::new();

        for span in &self.traces {
            let group = groups
                .entry(span.trace_id.clone())
                .or_insert_with(|| TraceGroup::new(span));
            update_trace_group(group, span);
        }

        let mut result: Vec<TraceGroup> = groups.into_values().collect();
        result.sort_by(|a, b| b.start_time.cmp(&a.start_time));
        result
    }

    pub fn sidebar_items(&self) -> Vec<SidebarItem> {
        let services = self
            .label_values
            .get("service")
            .cloned()
            .unwrap_or_default();
        let environments = self
            .label_values
            .get("environment")
            .cloned()
            .unwrap_or_default();

        if !services.is_empty() {
            build_service_sidebar(&services, &environments, &self.selected_labels)
        } else if !environments.is_empty() {
            build_environment_sidebar(&environments, &self.selected_labels)
        } else {
            Vec::new()
        }
    }

    pub fn select_current_log(&mut self) {
        let filtered = self.filtered_logs();
        if filtered.is_empty() {
            self.selected_log_idx = None;
            return;
        }
        let idx = self.log_cursor.min(filtered.len().saturating_sub(1));
        if self.selected_log_idx == Some(idx) {
            self.selected_log_idx = None;
        } else {
            self.selected_log_idx = Some(idx);
        }
    }

    pub fn selected_log_labels(&self) -> Option<&HashMap<String, String>> {
        let idx = self.selected_log_idx?;
        let filtered = self.filtered_logs();
        filtered.get(idx).map(|log| &log.labels)
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

fn build_service_sidebar(
    services: &[String],
    environments: &[String],
    selected: &HashMap<String, String>,
) -> Vec<SidebarItem> {
    let mut items = Vec::new();
    for svc in services {
        items.push(SidebarItem::Label(svc.clone()));
        items.push(SidebarItem::Value {
            label: "service".to_string(),
            value: svc.clone(),
            selected: selected.get("service") == Some(svc),
        });
        for env in environments {
            items.push(SidebarItem::Value {
                label: "environment".to_string(),
                value: env.clone(),
                selected: selected.get("environment") == Some(env),
            });
        }
    }
    items
}

fn build_environment_sidebar(
    environments: &[String],
    selected: &HashMap<String, String>,
) -> Vec<SidebarItem> {
    let mut items = vec![SidebarItem::Label("environment".to_string())];
    for env in environments {
        items.push(SidebarItem::Value {
            label: "environment".to_string(),
            value: env.clone(),
            selected: selected.get("environment") == Some(env),
        });
    }
    items
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

impl TraceGroup {
    fn new(span: &Span) -> Self {
        Self {
            trace_id: span.trace_id.clone(),
            root_operation: String::new(),
            root_service: String::new(),
            start_time: span.start_time,
            total_duration_ms: 0.0,
            span_count: 0,
            has_error: false,
        }
    }
}

fn update_trace_group(group: &mut TraceGroup, span: &Span) {
    group.span_count += 1;
    if span.parent_span_id.is_none() {
        group.root_operation = span.operation_name.clone();
        group.root_service = span.service_name.clone();
        group.total_duration_ms = span
            .end_time
            .signed_duration_since(span.start_time)
            .num_microseconds()
            .unwrap_or(0) as f64
            / 1000.0;
        group.start_time = span.start_time;
    }
    if span.status == common::span::SpanStatus::Error {
        group.has_error = true;
    }
}

pub enum SidebarItem {
    Label(String),
    Value {
        label: String,
        value: String,
        selected: bool,
    },
}

fn level_index(level: LogLevel) -> usize {
    match level {
        LogLevel::Trace => 0,
        LogLevel::Debug => 1,
        LogLevel::Info => 2,
        LogLevel::Warn => 3,
        LogLevel::Error => 4,
        LogLevel::Fatal => 5,
    }
}

pub fn level_name(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Trace => "TRACE",
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO",
        LogLevel::Warn => "WARN",
        LogLevel::Error => "ERROR",
        LogLevel::Fatal => "FATAL",
    }
}
