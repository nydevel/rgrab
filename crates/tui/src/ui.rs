use common::log::LogLevel;
use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs};

use crate::app::{App, InputMode, SidebarItem, Tab};

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

    let search_indicator = match app.input_mode {
        InputMode::Search => format!(" / {} ", app.search),
        InputMode::Normal if !app.search.is_empty() => format!(" /{} ", app.search),
        _ => String::new(),
    };

    let live = if app.live_tail { " Live" } else { "" };
    let limit_str = format!(" {} lines", app.limit);

    let header_block = Block::default()
        .title(format!(" rgrab {search_indicator}{live}{limit_str} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let tabs = Tabs::new(titles)
        .block(header_block)
        .highlight_style(Style::default().fg(Color::Yellow));

    f.render_widget(tabs, area);

    if app.input_mode == InputMode::Search {
        let cursor_x = area.x + 8 + app.search.len() as u16;
        let cursor_y = area.y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

fn draw_logs_body(f: &mut Frame, app: &App, area: Rect) {
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Min(40)])
        .split(area);

    draw_sidebar(f, app, h_chunks[0]);
    draw_log_list(f, app, h_chunks[1]);
}

fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.sidebar_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Labels ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let items = app.sidebar_items();
    let list_items: Vec<ListItem> =
        items
            .iter()
            .enumerate()
            .map(|(i, item)| match item {
                SidebarItem::Label(name) => {
                    let style = Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD);
                    ListItem::new(Line::from(Span::styled(name.clone(), style)))
                        .style(highlight_if(app.sidebar_focused, i, app.sidebar_scroll))
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
                        .style(highlight_if(app.sidebar_focused, i, app.sidebar_scroll))
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
    let start = app.log_scroll.min(filtered.len().saturating_sub(1));
    let end = (start + visible_height).min(filtered.len());

    let items: Vec<ListItem> = filtered[start..end]
        .iter()
        .map(|log| {
            let ts = log.timestamp.format("%H:%M:%S%.3f");
            let level_str = format!("{:5}", format!("{:?}", log.level).to_uppercase());
            let level_color = level_color(log.level);

            let line = Line::from(vec![
                Span::styled(format!("{ts} "), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{level_str} "), Style::default().fg(level_color)),
                Span::raw(&log.message),
            ]);

            ListItem::new(line)
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
            desc("switch tab"),
            key("j/k"),
            desc("scroll"),
            key("h/l"),
            desc("sidebar"),
            key("/"),
            desc("search"),
            key("Enter"),
            desc("select"),
            key("L"),
            desc("live"),
            key("r"),
            desc("refresh"),
            key("+/-"),
            desc("limit"),
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

    let status = match app.tab {
        Tab::Logs => {
            let total = filtered.len();
            format!(
                " {total} lines | error: {error_count} | warn: {warn_count} | info: {info_count} | debug: {debug_count} "
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
