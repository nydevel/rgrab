use common::span::SpanStatus;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::App;

const TIMING_BAR_WIDTH: usize = 10;
const SLOW_THRESHOLD_MS: f64 = 100.0;
const MEDIUM_THRESHOLD_MS: f64 = 10.0;
const OPERATION_WIDTH: usize = 20;
const SERVICE_WIDTH: usize = 12;
const TRACE_OPERATION_WIDTH: usize = 25;
const TRACE_SERVICE_WIDTH: usize = 15;
const TRACE_ID_PREVIEW_LEN: usize = 8;

pub fn draw_traces(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Traces ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let trace_groups = app.unique_traces();
    let visible_height = area.height.saturating_sub(2) as usize;
    let start = app.trace_scroll.min(trace_groups.len().saturating_sub(1));
    let end = (start + visible_height).min(trace_groups.len());

    let mut items: Vec<ListItem> = Vec::new();

    for (i, group) in trace_groups[start..end].iter().enumerate() {
        let is_selected = !app.sidebar_focused && (start + i) == app.trace_scroll;
        items.push(render_trace_group_item(group, is_selected));
        render_expanded_spans(app, group, &mut items);
    }

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_trace_group_item(group: &crate::app::TraceGroup, is_selected: bool) -> ListItem<'static> {
    let status_icon = if group.has_error {
        Span::styled("x ", Style::default().fg(Color::Red))
    } else {
        Span::styled("o ", Style::default().fg(Color::Green))
    };

    let op = Span::styled(
        format!(
            "{:<TRACE_OPERATION_WIDTH$}",
            truncate(&group.root_operation, TRACE_OPERATION_WIDTH)
        ),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let svc = Span::styled(
        format!(
            "{:<TRACE_SERVICE_WIDTH$}",
            truncate(&group.root_service, TRACE_SERVICE_WIDTH)
        ),
        Style::default().fg(Color::Cyan),
    );

    let dur = Span::styled(
        format!("{:>8.1}ms ", group.total_duration_ms),
        duration_color(group.total_duration_ms),
    );

    let tid = Span::styled(
        format!(
            "{}.. ",
            &group.trace_id[..TRACE_ID_PREVIEW_LEN.min(group.trace_id.len())]
        ),
        Style::default().fg(Color::DarkGray),
    );

    let ts = Span::styled(
        group.start_time.format("%H:%M:%S").to_string(),
        Style::default().fg(Color::DarkGray),
    );

    let span_count = Span::styled(
        format!(" ({} spans)", group.span_count),
        Style::default().fg(Color::DarkGray),
    );

    let line = Line::from(vec![status_icon, op, svc, dur, tid, ts, span_count]);
    let style = if is_selected {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default()
    };
    ListItem::new(line).style(style)
}

fn render_expanded_spans(
    app: &App,
    group: &crate::app::TraceGroup,
    items: &mut Vec<ListItem<'static>>,
) {
    if app.expanded_trace.as_deref() != Some(&group.trace_id) {
        return;
    }
    let Some(spans) = app.trace_spans.get(&group.trace_id) else {
        return;
    };

    let trace_start = group.start_time;
    let total_dur = group.total_duration_ms.max(0.001);

    for span_data in spans {
        items.push(ListItem::new(render_span_line(
            span_data,
            trace_start,
            total_dur,
        )));
    }
}

fn render_span_line(
    span: &common::span::Span,
    trace_start: chrono::DateTime<chrono::Utc>,
    total_dur_ms: f64,
) -> Line<'static> {
    let prefix = if span.parent_span_id.is_some() {
        "  |-- "
    } else {
        "  "
    };
    let status_color = span_status_color(span.status);
    let span_dur_ms = span_duration_ms(span);
    let offset_ms = span_offset_ms(span, trace_start);
    let bar = build_timing_bar(offset_ms, span_dur_ms, total_dur_ms);

    Line::from(vec![
        Span::raw(prefix.to_string()),
        Span::styled(
            format!(
                "{:<OPERATION_WIDTH$}",
                truncate(&span.operation_name, OPERATION_WIDTH)
            ),
            Style::default().fg(status_color),
        ),
        Span::styled(
            format!(
                "{:<SERVICE_WIDTH$}",
                truncate(&span.service_name, SERVICE_WIDTH)
            ),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!("{:>7.1}ms ", span_dur_ms),
            duration_color(span_dur_ms),
        ),
        Span::styled(bar, Style::default().fg(status_color)),
    ])
}

fn span_status_color(status: SpanStatus) -> Color {
    match status {
        SpanStatus::Ok => Color::Green,
        SpanStatus::Error => Color::Red,
        SpanStatus::Unset => Color::Gray,
    }
}

fn span_duration_ms(span: &common::span::Span) -> f64 {
    span.end_time
        .signed_duration_since(span.start_time)
        .num_microseconds()
        .unwrap_or(0) as f64
        / 1000.0
}

fn span_offset_ms(span: &common::span::Span, trace_start: chrono::DateTime<chrono::Utc>) -> f64 {
    span.start_time
        .signed_duration_since(trace_start)
        .num_microseconds()
        .unwrap_or(0) as f64
        / 1000.0
}

fn build_timing_bar(offset_ms: f64, span_dur_ms: f64, total_dur_ms: f64) -> String {
    let bar_start = ((offset_ms / total_dur_ms) * TIMING_BAR_WIDTH as f64) as usize;
    let bar_len = ((span_dur_ms / total_dur_ms) * TIMING_BAR_WIDTH as f64).ceil() as usize;
    let bar_start = bar_start.min(TIMING_BAR_WIDTH);
    let bar_len = bar_len.max(1).min(TIMING_BAR_WIDTH - bar_start);

    let mut bar = String::with_capacity(TIMING_BAR_WIDTH);
    for i in 0..TIMING_BAR_WIDTH {
        if i >= bar_start && i < bar_start + bar_len {
            bar.push('#');
        } else {
            bar.push('.');
        }
    }
    bar
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}..", &s[..max.saturating_sub(2)])
    } else {
        s.to_string()
    }
}

fn duration_color(ms: f64) -> Style {
    if ms > SLOW_THRESHOLD_MS {
        Style::default().fg(Color::Red)
    } else if ms > MEDIUM_THRESHOLD_MS {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    }
}
