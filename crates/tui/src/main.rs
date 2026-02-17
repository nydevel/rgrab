mod app;
mod client;
mod ui;
mod ui_traces;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use app::{App, InputMode, SidebarItem, Tab};
use client::ApiClient;

#[tokio::main]
async fn main() -> Result<()> {
    let server = parse_server_arg();
    let client = ApiClient::new(&server);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &client).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn parse_server_arg() -> String {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--server" => {
                if i + 1 < args.len() {
                    return args[i + 1].clone();
                }
            }
            s if s.starts_with("--server=") => {
                return s.trim_start_matches("--server=").to_string();
            }
            _ => {}
        }
        i += 1;
    }
    "http://localhost:3000".to_string()
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    client: &ApiClient,
) -> Result<()> {
    let mut app = App::new(&client.base_url());
    app.refresh(client).await;

    let refresh_interval = Duration::from_secs(2);
    let mut last_refresh = std::time::Instant::now();
    let poll_timeout = Duration::from_millis(50);

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if event::poll(poll_timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handle_input(&mut app, key.code, client).await;
                }
                Event::Mouse(mouse) => {
                    handle_mouse(&mut app, mouse, client).await;
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }

        if app.live_tail && last_refresh.elapsed() >= refresh_interval {
            app.refresh(client).await;
            last_refresh = std::time::Instant::now();
        }
    }

    Ok(())
}

async fn handle_input(app: &mut App, code: KeyCode, client: &ApiClient) {
    match app.input_mode {
        InputMode::Search => handle_search_input(app, code),
        InputMode::Normal => handle_normal_input(app, code, client).await,
    }
}

fn handle_search_input(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
            app.log_cursor = 0;
        }
        KeyCode::Backspace => {
            app.search.pop();
        }
        KeyCode::Char(c) => {
            app.search.push(c);
        }
        _ => {}
    }
}

async fn handle_normal_input(app: &mut App, code: KeyCode, client: &ApiClient) {
    match code {
        KeyCode::Char('q') => app.should_quit = true,

        KeyCode::Tab => {
            app.tab = match app.tab {
                Tab::Logs => Tab::Traces,
                Tab::Traces => Tab::Logs,
            };
        }

        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
            app.search.clear();
        }

        KeyCode::Char('j') | KeyCode::Down => scroll_down(app, 1),
        KeyCode::Char('k') | KeyCode::Up => scroll_up(app, 1),
        KeyCode::PageDown => scroll_down(app, page_size(app)),
        KeyCode::PageUp => scroll_up(app, page_size(app)),
        KeyCode::Home => scroll_to_top(app),
        KeyCode::End => scroll_to_end(app),

        KeyCode::Char('h') | KeyCode::Left => {
            app.sidebar_focused = true;
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app.sidebar_focused = false;
        }

        KeyCode::Char('1') => app.toggle_level(0), // TRACE
        KeyCode::Char('2') => app.toggle_level(1), // DEBUG
        KeyCode::Char('3') => app.toggle_level(2), // INFO
        KeyCode::Char('4') => app.toggle_level(3), // WARN
        KeyCode::Char('5') => app.toggle_level(4), // ERROR
        KeyCode::Char('6') => app.toggle_level(5), // FATAL

        KeyCode::Enter => handle_enter(app, client).await,

        KeyCode::Esc => {
            if app.selected_log_idx.is_some() {
                app.selected_log_idx = None;
            } else if app.expanded_trace.is_some() {
                app.expanded_trace = None;
            } else if !app.search.is_empty() {
                app.search.clear();
                app.log_cursor = 0;
            }
        }

        KeyCode::Char('L') => {
            app.live_tail = !app.live_tail;
        }

        KeyCode::Char('r') => {
            app.refresh(client).await;
        }

        KeyCode::Char('+') | KeyCode::Char('=') => {
            app.limit = (app.limit + 100).min(10000);
            app.refresh(client).await;
        }

        KeyCode::Char('-') => {
            app.limit = app.limit.saturating_sub(100).max(50);
            app.refresh(client).await;
        }

        KeyCode::Char('s') => {
            app.newest_first = !app.newest_first;
            app.log_cursor = 0;
        }

        _ => {}
    }
}

fn scroll_down(app: &mut App, count: usize) {
    if app.sidebar_focused {
        let max = app.sidebar_items().len().saturating_sub(1);
        app.sidebar_scroll = (app.sidebar_scroll + count).min(max);
    } else {
        match app.tab {
            Tab::Logs => {
                let max = app.filtered_logs().len().saturating_sub(1);
                app.log_cursor = (app.log_cursor + count).min(max);
            }
            Tab::Traces => {
                let max = app.unique_traces().len().saturating_sub(1);
                app.trace_scroll = (app.trace_scroll + count).min(max);
            }
        }
    }
}

fn scroll_up(app: &mut App, count: usize) {
    if app.sidebar_focused {
        app.sidebar_scroll = app.sidebar_scroll.saturating_sub(count);
    } else {
        match app.tab {
            Tab::Logs => app.log_cursor = app.log_cursor.saturating_sub(count),
            Tab::Traces => app.trace_scroll = app.trace_scroll.saturating_sub(count),
        }
    }
}

fn scroll_to_top(app: &mut App) {
    if app.sidebar_focused {
        app.sidebar_scroll = 0;
    } else {
        match app.tab {
            Tab::Logs => app.log_cursor = 0,
            Tab::Traces => app.trace_scroll = 0,
        }
    }
}

fn scroll_to_end(app: &mut App) {
    if app.sidebar_focused {
        app.sidebar_scroll = app.sidebar_items().len().saturating_sub(1);
    } else {
        match app.tab {
            Tab::Logs => app.log_cursor = app.filtered_logs().len().saturating_sub(1),
            Tab::Traces => app.trace_scroll = app.unique_traces().len().saturating_sub(1),
        }
    }
}

fn page_size(app: &App) -> usize {
    app.limit.min(20)
}

fn area_contains(area: ratatui::layout::Rect, col: u16, row: u16) -> bool {
    col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height
}

fn row_in_area(area: ratatui::layout::Rect, row: u16) -> Option<usize> {
    if row > area.y && row < area.y + area.height.saturating_sub(1) {
        Some((row - area.y - 1) as usize)
    } else {
        None
    }
}

async fn handle_mouse(app: &mut App, mouse: crossterm::event::MouseEvent, client: &ApiClient) {
    let col = mouse.column;
    let row = mouse.row;

    match mouse.kind {
        MouseEventKind::ScrollUp => handle_mouse_scroll(app, col, row, true),
        MouseEventKind::ScrollDown => handle_mouse_scroll(app, col, row, false),
        MouseEventKind::Down(MouseButton::Left) => handle_mouse_click(app, col, row, client).await,
        _ => {}
    }
}

fn handle_mouse_scroll(app: &mut App, col: u16, row: u16, up: bool) {
    if area_contains(app.areas.sidebar, col, row) {
        if up {
            app.sidebar_scroll = app.sidebar_scroll.saturating_sub(3);
        } else {
            let max = app.sidebar_items().len().saturating_sub(1);
            app.sidebar_scroll = (app.sidebar_scroll + 3).min(max);
        }
    } else if area_contains(app.areas.log_list, col, row) && app.tab == Tab::Logs {
        if up {
            app.log_cursor = app.log_cursor.saturating_sub(3);
        } else {
            let max = app.filtered_logs().len().saturating_sub(1);
            app.log_cursor = (app.log_cursor + 3).min(max);
        }
    } else if area_contains(app.areas.traces, col, row) && app.tab == Tab::Traces {
        if up {
            app.trace_scroll = app.trace_scroll.saturating_sub(3);
        } else {
            let max = app.unique_traces().len().saturating_sub(1);
            app.trace_scroll = (app.trace_scroll + 3).min(max);
        }
    }
}

async fn handle_mouse_click(app: &mut App, col: u16, row: u16, client: &ApiClient) {
    if area_contains(app.areas.header, col, row) {
        handle_header_click(app, col);
        return;
    }

    if area_contains(app.areas.level_tabs, col, row) {
        handle_level_click(app, col);
        return;
    }

    if area_contains(app.areas.sidebar, col, row) {
        app.sidebar_focused = true;
        if let Some(idx) = row_in_area(app.areas.sidebar, row) {
            app.sidebar_scroll = idx;
            let items = app.sidebar_items();
            if let Some(SidebarItem::Value { label, value, .. }) = items.get(idx) {
                let label = label.clone();
                let value = value.clone();
                app.toggle_label(&label, &value);
                app.log_cursor = 0;
            }
        }
        return;
    }

    if area_contains(app.areas.log_list, col, row) && app.tab == Tab::Logs {
        app.sidebar_focused = false;
        if let Some(idx) = row_in_area(app.areas.log_list, row) {
            let start = if app.log_cursor >= app.areas.log_list.height.saturating_sub(2) as usize {
                app.log_cursor - app.areas.log_list.height.saturating_sub(2) as usize + 1
            } else {
                0
            };
            let clicked_log = start + idx;
            let max = app.filtered_logs().len().saturating_sub(1);
            app.log_cursor = clicked_log.min(max);
            app.select_current_log();
        }
        return;
    }

    if area_contains(app.areas.traces, col, row) && app.tab == Tab::Traces {
        app.sidebar_focused = false;
        if let Some(idx) = row_in_area(app.areas.traces, row) {
            let clicked_trace = app
                .trace_scroll
                .min(app.unique_traces().len().saturating_sub(1))
                + idx;
            let max = app.unique_traces().len().saturating_sub(1);
            app.trace_scroll = clicked_trace.min(max);
            handle_enter(app, client).await;
        }
    }
}

fn handle_level_click(app: &mut App, col: u16) {
    let rel = col.saturating_sub(app.areas.level_tabs.x) as usize;
    let mut offset = 0;
    for (i, &level) in app::ALL_LEVELS.iter().enumerate() {
        let name_len = app::level_name(level).len();
        let tab_width = 3 + name_len + 1; // " N " + "NAME "
        if rel < offset + tab_width {
            app.toggle_level(i);
            return;
        }
        offset += tab_width;
    }
}

fn handle_header_click(app: &mut App, col: u16) {
    let area = app.areas.header;
    let rel_col = col.saturating_sub(area.x + 1);
    if rel_col < 8 {
        app.tab = Tab::Logs;
    } else {
        app.tab = Tab::Traces;
    }
}

async fn handle_enter(app: &mut App, client: &ApiClient) {
    if app.sidebar_focused {
        let items = app.sidebar_items();
        if let Some(SidebarItem::Value { label, value, .. }) = items.get(app.sidebar_scroll) {
            let label = label.clone();
            let value = value.clone();
            app.toggle_label(&label, &value);
            app.log_cursor = 0;
        }
    } else if app.tab == Tab::Logs {
        app.select_current_log();
    } else if app.tab == Tab::Traces {
        let groups = app.unique_traces();
        if let Some(group) = groups.get(app.trace_scroll) {
            let trace_id = group.trace_id.clone();
            if app.expanded_trace.as_deref() == Some(&trace_id) {
                app.expanded_trace = None;
            } else {
                app.expand_trace(client, &trace_id).await;
            }
        }
    }
}
