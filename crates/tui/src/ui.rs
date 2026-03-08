use common::log::LogLevel;
use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs};

use crate::app::{self, App, InputMode, SidebarItem, Tab};

const SERVER_PANEL_WIDTH: u16 = 6;

pub fn draw(f: &mut Frame, app: &mut App) {
    let top_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(SERVER_PANEL_WIDTH), Constraint::Min(40)])
        .split(f.area());

    app.areas.server_panel = top_split[0];
    draw_server_panel(f, app, top_split[0]);

    let main_area = top_split[1];
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(main_area);

    app.areas.header = chunks[0];

    draw_header(f, app, chunks[0]);

    match app.tab {
        Tab::Logs => draw_logs_body(f, app, chunks[1]),
        Tab::Traces => {
            app.areas.traces = chunks[1];
            super::ui_traces::draw_traces(f, app, chunks[1]);
        }
    }

    draw_footer(f, app, chunks[2]);

    if !app.connected {
        draw_disconnected(f, app);
    }
}

fn draw_server_panel(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .servers
        .iter()
        .enumerate()
        .map(|(i, _)| render_server_item(i, i == app.active_server))
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(list, area);
}

fn render_server_item(index: usize, active: bool) -> ListItem<'static> {
    let icon = format!(" \u{1F5B3} {} ", index + 1);
    let style = if active {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    ListItem::new(Line::from(Span::styled(icon, style)))
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
    let loaded_str = if app.has_more {
        format!(" {}+ loaded", app.logs.len())
    } else {
        format!(" {} loaded", app.logs.len())
    };
    let server_name = app.server_name();

    let version = env!("CARGO_PKG_VERSION");
    let header_block = Block::default()
        .title(format!(
            " rgrab v{version} [{server_name}]{live}{loaded_str} "
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let selected = match app.tab {
        Tab::Logs => 0,
        Tab::Traces => 1,
    };

    let tabs = Tabs::new(titles)
        .block(header_block)
        .select(selected)
        .highlight_style(Style::default().fg(Color::Yellow));

    f.render_widget(tabs, area);
}

fn draw_logs_body(f: &mut Frame, app: &mut App, area: Rect) {
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(4),
        ])
        .split(area);

    app.areas.level_tabs = v_chunks[0];
    draw_level_tabs(f, app, v_chunks[0]);
    draw_search_bar(f, app, v_chunks[1]);

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Min(40)])
        .split(v_chunks[2]);

    app.areas.sidebar = h_chunks[0];
    app.areas.log_list = h_chunks[1];

    draw_sidebar(f, app, h_chunks[0]);
    draw_log_list(f, app, h_chunks[1]);
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.input_mode == InputMode::Search;
    let border_style = search_border_style(is_active);
    let (text, text_style) = search_text(app, is_active);

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

fn search_border_style(active: bool) -> Style {
    if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn search_text(app: &App, active: bool) -> (String, Style) {
    if active {
        (
            format!("{}_", app.search),
            Style::default().fg(Color::Yellow),
        )
    } else if app.search.is_empty() {
        (
            "search...".to_string(),
            Style::default().fg(Color::DarkGray),
        )
    } else {
        (app.search.clone(), Style::default().fg(Color::White))
    }
}

fn draw_level_tabs(f: &mut Frame, app: &App, area: Rect) {
    let spans: Vec<Span> = app::ALL_LEVELS
        .iter()
        .enumerate()
        .flat_map(|(i, &level)| render_level_tab(i, level, app.level_enabled[i]))
        .collect();

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_level_tab(index: usize, level: LogLevel, enabled: bool) -> Vec<Span<'static>> {
    let name = app::level_name(level);
    let key_span = Span::styled(
        format!(" {} ", index + 1),
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
        .map(|(k, v)| render_label_detail(k, v))
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_label_detail<'a>(key: &str, value: &str) -> ListItem<'a> {
    let line = Line::from(vec![
        Span::styled(
            format!("{key}: "),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ]);
    ListItem::new(line)
}

fn draw_filter_sidebar(f: &mut Frame, app: &App, border_style: Style, area: Rect) {
    let block = Block::default()
        .title(" Labels ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let items = app.sidebar_items();
    let visible_height = area.height.saturating_sub(2) as usize;
    let scroll_offset =
        compute_scroll_offset(app.sidebar_focused, app.sidebar_scroll, visible_height);
    let end = (scroll_offset + visible_height).min(items.len());

    let list_items: Vec<ListItem> = items[scroll_offset..end]
        .iter()
        .enumerate()
        .map(|(vi, item)| {
            render_sidebar_item(
                item,
                app.sidebar_focused,
                scroll_offset + vi,
                app.sidebar_scroll,
            )
        })
        .collect();

    let list = List::new(list_items).block(block);
    f.render_widget(list, area);
}

fn compute_scroll_offset(focused: bool, cursor: usize, visible_height: usize) -> usize {
    if focused && cursor >= visible_height {
        cursor - visible_height + 1
    } else {
        0
    }
}

fn render_sidebar_item(
    item: &SidebarItem,
    focused: bool,
    global_idx: usize,
    scroll: usize,
) -> ListItem<'static> {
    match item {
        SidebarItem::Label(name) => {
            let style = Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
            ListItem::new(Line::from(Span::styled(name.clone(), style)))
                .style(highlight_if(focused, global_idx, scroll))
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
                .style(highlight_if(focused, global_idx, scroll))
        }
    }
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
    let start = compute_scroll_offset(true, cursor, visible_height);
    let end = (start + visible_height).min(filtered.len());

    let items: Vec<ListItem> = filtered[start..end]
        .iter()
        .enumerate()
        .map(|(vi, log)| {
            render_log_item(
                log,
                start + vi,
                cursor,
                app.sidebar_focused,
                app.selected_log_idx,
            )
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_log_item(
    log: &common::log::LogEntry,
    global_idx: usize,
    cursor: usize,
    sidebar_focused: bool,
    selected_log_idx: Option<usize>,
) -> ListItem<'static> {
    let ts = log.timestamp.format("%H:%M:%S%.3f");
    let level_str = format!("{:5}", format!("{:?}", log.level).to_uppercase());
    let lc = level_color(log.level);

    let line = Line::from(vec![
        Span::styled(format!("{ts} "), Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{level_str} "), Style::default().fg(lc)),
        Span::raw(log.message.clone()),
    ]);

    let is_selected = selected_log_idx == Some(global_idx);
    let is_cursor = !sidebar_focused && global_idx == cursor;
    let style = if is_selected {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    } else if is_cursor {
        Style::default().bg(Color::Indexed(236))
    } else {
        Style::default()
    };

    ListItem::new(line).style(style)
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
    let hints = build_keyhints(app.input_mode);
    f.render_widget(Paragraph::new(Line::from(hints)), area);
}

fn hint_key(k: &str) -> Span<'static> {
    Span::styled(
        format!(" {k} "),
        Style::default().fg(Color::Black).bg(Color::DarkGray),
    )
}

fn hint_desc(d: &str) -> Span<'static> {
    Span::styled(format!(" {d}  "), Style::default().fg(Color::DarkGray))
}

fn build_keyhints(mode: InputMode) -> Vec<Span<'static>> {
    match mode {
        InputMode::Search => vec![
            hint_key("Enter"),
            hint_desc("apply"),
            hint_key("Esc"),
            hint_desc("cancel"),
        ],
        InputMode::Normal => vec![
            hint_key("Tab"),
            hint_desc("tab"),
            hint_key("^/v"),
            hint_desc("scroll"),
            hint_key("PgUp/Dn"),
            hint_desc("page"),
            hint_key("</>"),
            hint_desc("sidebar"),
            hint_key("/"),
            hint_desc("search"),
            hint_key("1-6"),
            hint_desc("filter"),
            hint_key("Enter"),
            hint_desc("select"),
            hint_key("s"),
            hint_desc("sort"),
            hint_key("C-N"),
            hint_desc("server"),
            hint_key("L"),
            hint_desc("live"),
            hint_key("r"),
            hint_desc("refresh"),
            hint_key("q"),
            hint_desc("quit"),
        ],
    }
}

struct LevelCounts {
    error: usize,
    warn: usize,
    info: usize,
    debug: usize,
}

fn count_levels(logs: &[&common::log::LogEntry]) -> LevelCounts {
    let mut counts = LevelCounts {
        error: 0,
        warn: 0,
        info: 0,
        debug: 0,
    };
    for log in logs {
        match log.level {
            LogLevel::Error | LogLevel::Fatal => counts.error += 1,
            LogLevel::Warn => counts.warn += 1,
            LogLevel::Info => counts.info += 1,
            LogLevel::Debug | LogLevel::Trace => counts.debug += 1,
        }
    }
    counts
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status = format_status_text(app);
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

fn format_status_text(app: &App) -> String {
    match app.tab {
        Tab::Logs => {
            let filtered = app.filtered_logs();
            let counts = count_levels(&filtered);
            let sort_order = if app.newest_first { "newest" } else { "oldest" };
            format!(
                " {} lines | {sort_order} first | error: {} | warn: {} | info: {} | debug: {} ",
                filtered.len(),
                counts.error,
                counts.warn,
                counts.info,
                counts.debug,
            )
        }
        Tab::Traces => format!(" {} traces ", app.unique_traces().len()),
    }
}

fn centered_popup(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width.saturating_sub(4));
    let h = height.min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

fn disconnected_text(server_url: &str, error_detail: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            "No connection to server",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            server_url.to_string(),
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            error_detail.to_string(),
            Style::default().fg(Color::DarkGray),
        )),
    ]
}

fn draw_disconnected(f: &mut Frame, app: &App) {
    let popup = centered_popup(f.area(), 50, 7);
    let error_detail = app.error.as_deref().unwrap_or("connection refused");
    let text = disconnected_text(app.server_url(), error_detail);

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
