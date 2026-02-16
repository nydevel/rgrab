use common::log::LogLevel;
use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs};

use crate::app::{self, App, InputMode, SidebarItem, Tab};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);

    match app.tab {
        Tab::Logs => draw_logs_body(f, app, chunks[1]),
        Tab::Traces => super::ui_traces::draw_traces(f, app, chunks[1]),
    }

    draw_footer(f, app, chunks[2]);

    if !app.connected {
        draw_disconnected(f, app);
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec![
        Line::from(if app.tab == Tab::Logs {
            Span::styled("[Logs]", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("Logs")
        }),
        Line::from(if app.tab == Tab::Traces {
            Span::styled("[Traces]", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("Traces")
        }),
    ];

    let live = if app.live_tail { " Live" } else { "" };
    let limit_str = format!(" {} lines", app.limit);

    let header_block = Block::default()
        .title(format!(" rgrab{live}{limit_str} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let tabs = Tabs::new(titles)
        .block(header_block)
        .highlight_style(Style::default().fg(Color::Yellow));

    f.render_widget(tabs, area);
}

fn draw_logs_body(f: &mut Frame, app: &App, area: Rect) {
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(4),
        ])
        .split(area);

    draw_level_tabs(f, app, v_chunks[0]);
    draw_search_bar(f, app, v_chunks[1]);

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Min(40)])
        .split(v_chunks[2]);

    draw_sidebar(f, app, h_chunks[0]);
    draw_log_list(f, app, h_chunks[1]);
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.input_mode == InputMode::Search;

    let border_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let text_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else if app.search.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let text = if is_active {
        format!("{}_", app.search)
    } else if app.search.is_empty() {
        "search...".to_string()
    } else {
        app.search.clone()
    };

    let block = Block::default()
        .title(" / ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let paragraph = Paragraph::new(Line::from(Span::styled(text, text_style))).block(block);
    f.render_widget(paragraph, area);

    if is_active {
        let cursor_x = area.x + 1 + app.search.len() as u16;
        f.set_cursor_position((cursor_x, area.y + 1));
    }
}

fn draw_level_tabs(f: &mut Frame, app: &App, area: Rect) {
    let level_color = |level: common::log::LogLevel| -> Color {
        match level {
            common::log::LogLevel::Trace => Color::DarkGray,
            common::log::LogLevel::Debug => Color::Gray,
            common::log::LogLevel::Info => Color::Blue,
            common::log::LogLevel::Warn => Color::Yellow,
            common::log::LogLevel::Error => Color::Red,
            common::log::LogLevel::Fatal => Color::LightRed,
        }
    };

    let spans: Vec<Span> = app::ALL_LEVELS
        .iter()
        .enumerate()
        .flat_map(|(i, &level)| {
            let name = app::level_name(level);
            let enabled = app.level_enabled[i];
            let key_span = Span::styled(
                format!(" {} ", i + 1),
                Style::default().fg(Color::Black).bg(Color::DarkGray),
            );
            let label_span = if enabled {
                Span::styled(
                    format!("{name} "),
                    Style::default()
                        .fg(level_color(level))
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    format!("{name} "),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                )
            };
            vec![key_span, label_span]
        })
        .collect();

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.sidebar_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    if let Some(log_labels) = app.selected_log_labels() {
        draw_log_detail_sidebar(f, log_labels, border_style, area);
    } else {
        draw_filter_sidebar(f, app, border_style, area);
    }
}

fn draw_log_detail_sidebar(
    f: &mut Frame,
    labels: &std::collections::HashMap<String, String>,
    border_style: Style,
    area: Rect,
) {
    let block = Block::default()
        .title(" Log Labels ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let mut sorted: Vec<(&String, &String)> = labels.iter().collect();
    sorted.sort_by_key(|(k, _)| k.as_str());

    let items: Vec<ListItem> = sorted
        .iter()
        .map(|(k, v)| {
            let line = Line::from(vec![
                Span::styled(
                    format!("{k}: "),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(v.to_string(), Style::default().fg(Color::White)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_filter_sidebar(f: &mut Frame, app: &App, border_style: Style, area: Rect) {
    let block = Block::default()
        .title(" Labels ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let items = app.sidebar_items();
    let visible_height = area.height.saturating_sub(2) as usize;

    let scroll_offset = if app.sidebar_focused {
        let cursor = app.sidebar_scroll;
        if cursor >= visible_height {
            cursor - visible_height + 1
        } else {
            0
        }
    } else {
        0
    };

    let end = (scroll_offset + visible_height).min(items.len());
    let list_items: Vec<ListItem> = items[scroll_offset..end]
        .iter()
        .enumerate()
        .map(|(vi, item)| {
            let global_idx = scroll_offset + vi;
            match item {
                SidebarItem::Label(name) => {
                    let style = Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD);
                    ListItem::new(Line::from(Span::styled(name.clone(), style))).style(
                        highlight_if(app.sidebar_focused, global_idx, app.sidebar_scroll),
                    )
                }
                SidebarItem::Value {
                    value, selected, ..
                } => {
                    let prefix = if *selected { " > " } else { "   " };
                    let style = if *selected {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(Line::from(Span::styled(format!("{prefix}{value}"), style)))
                        .style(highlight_if(
                            app.sidebar_focused,
                            global_idx,
                            app.sidebar_scroll,
                        ))
                }
            }
        })
        .collect();

    let list = List::new(list_items).block(block);
    f.render_widget(list, area);
}

fn highlight_if(focused: bool, index: usize, scroll: usize) -> Style {
    if focused && index == scroll {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default()
    }
}

fn draw_log_list(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Logs ")
        .borders(Borders::ALL)
        .border_style(if app.sidebar_focused {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Yellow)
        });

    let filtered = app.filtered_logs();
    let visible_height = area.height.saturating_sub(2) as usize;
    let cursor = app.log_cursor.min(filtered.len().saturating_sub(1));

    let start = if cursor >= visible_height {
        cursor - visible_height + 1
    } else {
        0
    };
    let end = (start + visible_height).min(filtered.len());

    let items: Vec<ListItem> = filtered[start..end]
        .iter()
        .enumerate()
        .map(|(vi, log)| {
            let global_idx = start + vi;
            let ts = log.timestamp.format("%H:%M:%S%.3f");
            let level_str = format!("{:5}", format!("{:?}", log.level).to_uppercase());
            let lc = level_color(log.level);

            let line = Line::from(vec![
                Span::styled(format!("{ts} "), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{level_str} "), Style::default().fg(lc)),
                Span::raw(&log.message),
            ]);

            let is_selected = app.selected_log_idx == Some(global_idx);
            let is_cursor = !app.sidebar_focused && global_idx == cursor;
            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else if is_cursor {
                Style::default().bg(Color::Indexed(236))
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn level_color(level: LogLevel) -> Color {
    match level {
        LogLevel::Trace => Color::DarkGray,
        LogLevel::Debug => Color::Gray,
        LogLevel::Info => Color::Blue,
        LogLevel::Warn => Color::Yellow,
        LogLevel::Error => Color::Red,
        LogLevel::Fatal => Color::LightRed,
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(2)])
        .split(area);

    draw_keyhints(f, app, chunks[0]);
    draw_status_bar(f, app, chunks[1]);
}

fn draw_keyhints(f: &mut Frame, app: &App, area: Rect) {
    let key = |k: &str| {
        Span::styled(
            format!(" {k} "),
            Style::default().fg(Color::Black).bg(Color::DarkGray),
        )
    };
    let desc = |d: &str| Span::styled(format!(" {d}"), Style::default().fg(Color::DarkGray));

    let hints: Vec<Span> = match app.input_mode {
        InputMode::Search => vec![key("Enter"), desc("apply"), key("Esc"), desc("cancel")],
        InputMode::Normal => vec![
            key("Tab"),
            desc("tab"),
            key("^/v"),
            desc("scroll"),
            key("PgUp/Dn"),
            desc("page"),
            key("</>"),
            desc("sidebar"),
            key("/"),
            desc("search"),
            key("1-6"),
            desc("filter"),
            key("Enter"),
            desc("select"),
            key("s"),
            desc("sort"),
            key("L"),
            desc("live"),
            key("r"),
            desc("refresh"),
            key("q"),
            desc("quit"),
        ],
    };

    let line = Line::from(hints);
    f.render_widget(Paragraph::new(line), area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let filtered = app.filtered_logs();

    let mut error_count = 0usize;
    let mut warn_count = 0usize;
    let mut info_count = 0usize;
    let mut debug_count = 0usize;

    for log in &filtered {
        match log.level {
            LogLevel::Error | LogLevel::Fatal => error_count += 1,
            LogLevel::Warn => warn_count += 1,
            LogLevel::Info => info_count += 1,
            LogLevel::Debug | LogLevel::Trace => debug_count += 1,
        }
    }

    let sort_order = if app.newest_first { "newest" } else { "oldest" };

    let status = match app.tab {
        Tab::Logs => {
            let total = filtered.len();
            format!(
                " {total} lines | {sort_order} first | error: {error_count} | warn: {warn_count} | info: {info_count} | debug: {debug_count} "
            )
        }
        Tab::Traces => {
            let traces = app.unique_traces();
            format!(" {} traces ", traces.len())
        }
    };

    let error_text = app
        .error
        .as_ref()
        .map(|e| format!(" | err: {e}"))
        .unwrap_or_default();

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(status, Style::default().fg(Color::DarkGray)),
        Span::styled(error_text, Style::default().fg(Color::Red)),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(footer, area);
}

fn draw_disconnected(f: &mut Frame, app: &App) {
    let area = f.area();
    let width = 50u16.min(area.width.saturating_sub(4));
    let height = 7u16.min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    let error_detail = app.error.as_deref().unwrap_or("connection refused");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "No connection to server",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.server_url.as_str(),
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            error_detail,
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .title(" Disconnected ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(Clear, popup);
    f.render_widget(paragraph, popup);
}
