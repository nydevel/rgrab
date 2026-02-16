use common::span::SpanStatus;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::App;

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
        let global_idx = start + i;
        let is_selected = !app.sidebar_focused && global_idx == app.trace_scroll;

        let status_icon = if group.has_error {
            Span::styled("x ", Style::default().fg(Color::Red))
        } else {
            Span::styled("o ", Style::default().fg(Color::Green))
        };

        let op = Span::styled(
            format!("{:<25}", truncate(&group.root_operation, 25)),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

        let svc = Span::styled(
            format!("{:<15}", truncate(&group.root_service, 15)),
            Style::default().fg(Color::Cyan),
        );

        let dur = Span::styled(
            format!("{:>8.1}ms ", group.total_duration_ms),
            duration_color(group.total_duration_ms),
        );

        let tid = Span::styled(
            format!("{}.. ", &group.trace_id[..8.min(group.trace_id.len())]),
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
        items.push(ListItem::new(line).style(style));

        if app.expanded_trace.as_deref() == Some(&group.trace_id)
            && let Some(spans) = app.trace_spans.get(&group.trace_id)
        {
            let trace_start = group.start_time;
            let total_dur = group.total_duration_ms.max(0.001);

            for span_data in spans {
                let span_items = render_span_line(span_data, trace_start, total_dur);
                items.push(ListItem::new(span_items));
            }
        }
    }

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_span_line(
    span: &common::span::Span,
    trace_start: chrono::DateTime<chrono::Utc>,
    total_dur_ms: f64,
) -> Line<'static> {
    let is_child = span.parent_span_id.is_some();
    let prefix = if is_child { "  |-- " } else { "  " };

    let status_color = match span.status {
        SpanStatus::Ok => Color::Green,
        SpanStatus::Error => Color::Red,
        SpanStatus::Unset => Color::Gray,
    };

    let span_dur_ms = span
        .end_time
        .signed_duration_since(span.start_time)
        .num_microseconds()
        .unwrap_or(0) as f64
        / 1000.0;

    let offset_ms = span
        .start_time
        .signed_duration_since(trace_start)
        .num_microseconds()
        .unwrap_or(0) as f64
        / 1000.0;

    let bar_width = 10;
    let bar_start = ((offset_ms / total_dur_ms) * bar_width as f64) as usize;
    let bar_len = ((span_dur_ms / total_dur_ms) * bar_width as f64).ceil() as usize;
    let bar_start = bar_start.min(bar_width);
    let bar_len = bar_len.max(1).min(bar_width - bar_start);

    let mut bar = String::with_capacity(bar_width);
    for i in 0..bar_width {
        if i >= bar_start && i < bar_start + bar_len {
            bar.push('#');
        } else {
            bar.push('.');
        }
    }

    Line::from(vec![
        Span::raw(prefix.to_string()),
        Span::styled(
            format!("{:<20}", truncate(&span.operation_name, 20)),
            Style::default().fg(status_color),
        ),
        Span::styled(
            format!("{:<12}", truncate(&span.service_name, 12)),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!("{:>7.1}ms ", span_dur_ms),
            duration_color(span_dur_ms),
        ),
        Span::styled(bar, Style::default().fg(status_color)),
    ])
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}..", &s[..max.saturating_sub(2)])
    } else {
        s.to_string()
    }
}

fn duration_color(ms: f64) -> Style {
    if ms > 100.0 {
        Style::default().fg(Color::Red)
    } else if ms > 10.0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    }
}
