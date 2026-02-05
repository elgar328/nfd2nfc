use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use crossterm::event::{Event, KeyCode, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use crate::daemon_controller;
use crate::tui::app::state::{App, PendingWatcherOperation};
use crate::tui::component::{Action, ScrollDirection, TabComponent};
use crate::tui::tabs::Tab;

pub enum ClickResult {
    DoubleClick,
    AreaAction(KeyCode),
    PassThrough,
}

/// Represents a clickable area with its associated key action
#[derive(Debug, Clone)]
pub struct ClickableArea {
    pub rect: Rect,
    pub action: KeyCode,
}

/// Double-click threshold in milliseconds
const DOUBLE_CLICK_THRESHOLD_MS: u128 = 300;

/// Unified mouse state: click areas and double-click detection
#[derive(Default)]
pub struct MouseState {
    areas: Vec<ClickableArea>,
    last_click: Option<(u16, u16, Instant)>,
}

impl MouseState {
    pub fn clear(&mut self) {
        self.areas.clear();
    }

    pub fn add(&mut self, rect: Rect, action: KeyCode) {
        self.areas.push(ClickableArea { rect, action });
    }

    pub fn find_at(&self, x: u16, y: u16) -> Option<&KeyCode> {
        let pos = Position { x, y };
        self.areas
            .iter()
            .find(|area| area.rect.contains(pos))
            .map(|area| &area.action)
    }

    /// Records a click and returns whether it forms a double-click with the previous one.
    fn detect_double_click(&mut self, x: u16, y: u16) -> bool {
        let now = Instant::now();
        let is_double = self.last_click.is_some_and(|(lx, ly, lt)| {
            lx == x && ly == y && now.duration_since(lt).as_millis() < DOUBLE_CLICK_THRESHOLD_MS
        });

        self.last_click = if is_double { None } else { Some((x, y, now)) };
        is_double
    }

    /// Register click areas from a list of (spans, optional key) items,
    /// automatically computing x positions from span widths.
    /// Returns the flattened spans for rendering.
    pub fn add_shortcuts<'a>(
        &mut self,
        items: Vec<(Vec<Span<'a>>, Option<KeyCode>)>,
        start_x: u16,
        y: u16,
    ) -> Vec<Span<'a>> {
        let mut x = start_x;
        let mut all_spans = Vec::new();
        for (spans, key) in items {
            let width: u16 = spans.iter().map(|s| s.content.width() as u16).sum();
            if let Some(key) = key {
                self.add(Rect::new(x, y, width, 1), key);
            }
            x += width;
            all_spans.extend(spans);
        }
        all_spans
    }

    pub fn resolve_click(&mut self, x: u16, y: u16) -> ClickResult {
        if self.detect_double_click(x, y) {
            ClickResult::DoubleClick
        } else if let Some(action) = self.find_at(x, y).cloned() {
            ClickResult::AreaAction(action)
        } else {
            ClickResult::PassThrough
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Common key handling (was in tabs/mod.rs)
// ─────────────────────────────────────────────────────────────

/// Handles common keys shared across all tabs (quit, tab switch, number keys).
/// Returns Some(Action) if a common key was handled.
fn handle_common_key(key: KeyCode) -> Option<Action> {
    if let Some(tab) = Tab::from_key(key) {
        return Some(Action::SelectTab(tab));
    }
    match key {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Tab => Some(Action::NextTab),
        KeyCode::BackTab => Some(Action::PreviousTab),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────
// Event handling
// ─────────────────────────────────────────────────────────────

pub fn handle_event(
    app: &mut App,
    maybe_event: Option<Result<Event, std::io::Error>>,
) -> Result<(), std::io::Error> {
    match maybe_event {
        Some(Ok(Event::Key(key))) => handle_key(app, key.code),
        Some(Ok(Event::Mouse(mouse))) => handle_mouse(app, mouse),
        Some(Err(e)) => return Err(e),
        _ => {}
    }
    Ok(())
}

pub fn handle_key(app: &mut App, key: KeyCode) {
    // 1. Let current tab handle first
    let shared = app.shared_state();
    let action = match app.current_tab {
        Tab::Home => app.home.handle_key(key, &shared),
        Tab::Config => app.config.handle_key(key, &shared),
        Tab::Logs => app.logs.handle_key(key, &shared),
        Tab::Browser => app.browser.handle_key(key, &shared),
    };

    if let Some(action) = action {
        process_action(app, action);
        return;
    }

    // 2. Common keys (only reached if tab didn't handle)
    if let Some(action) = handle_common_key(key) {
        process_action(app, action);
    }
}

fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    let x = mouse.column;
    let y = mouse.row;

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => match app.mouse_state.resolve_click(x, y) {
            ClickResult::DoubleClick => dispatch_double_click(app, x, y),
            ClickResult::AreaAction(action) => handle_key(app, action),
            ClickResult::PassThrough => dispatch_click(app, x, y),
        },
        MouseEventKind::ScrollUp => dispatch_scroll(app, ScrollDirection::Up),
        MouseEventKind::ScrollDown => dispatch_scroll(app, ScrollDirection::Down),
        _ => {}
    }
}

fn dispatch_double_click(app: &mut App, x: u16, y: u16) {
    let action = match app.current_tab {
        Tab::Config => app.config.handle_double_click(x, y),
        Tab::Browser => app.browser.handle_double_click(x, y),
        _ => None,
    };
    if let Some(action) = action {
        process_action(app, action);
    }
}

fn dispatch_click(app: &mut App, x: u16, y: u16) {
    let action = match app.current_tab {
        Tab::Config => app.config.handle_mouse_click(x, y),
        Tab::Browser => app.browser.handle_mouse_click(x, y),
        Tab::Logs => app.logs.handle_mouse_click(x, y),
        _ => None,
    };
    if let Some(action) = action {
        process_action(app, action);
    }
}

fn dispatch_scroll(app: &mut App, direction: ScrollDirection) {
    let action = match app.current_tab {
        Tab::Home => None,
        Tab::Config => app.config.handle_scroll(direction),
        Tab::Logs => app.logs.handle_scroll(direction),
        Tab::Browser => app.browser.handle_scroll(direction),
    };
    if let Some(action) = action {
        process_action(app, action);
    }
}

// ─────────────────────────────────────────────────────────────
// Action processing
// ─────────────────────────────────────────────────────────────

fn spawn_watcher_op(app: &mut App, op: PendingWatcherOperation, f: fn() -> Result<(), String>) {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(f());
    });
    app.start_async_operation(op, rx);
}

pub fn process_action(app: &mut App, action: Action) {
    match action {
        Action::Consumed => {}
        Action::Quit => app.quit(),
        Action::NextTab => app.next_tab(),
        Action::PreviousTab => app.previous_tab(),
        Action::SelectTab(tab) => app.select_tab(tab),
        Action::ShowToast { message, is_error } => app.show_toast(message, is_error),
        Action::StartWatcher => {
            spawn_watcher_op(
                app,
                PendingWatcherOperation::Starting,
                daemon_controller::try_start_watcher,
            );
        }
        Action::StopWatcher => {
            spawn_watcher_op(
                app,
                PendingWatcherOperation::Stopping,
                daemon_controller::try_stop_watcher,
            );
        }
        Action::RestartWatcher => {
            spawn_watcher_op(
                app,
                PendingWatcherOperation::Restarting,
                daemon_controller::try_restart_watcher,
            );
        }
        Action::ConfigSaved => {
            app.show_toast("Config saved".to_string(), false);
            if app.watcher_running {
                spawn_watcher_op(
                    app,
                    PendingWatcherOperation::Restarting,
                    daemon_controller::try_restart_watcher,
                );
            }
        }
        Action::ReloadConfig => {
            app.config.reload();
            app.show_toast("Changes cancelled".to_string(), false);
        }
    }
}
