use std::sync::mpsc::Receiver;

use nfd2nfc_core::config::load_config;
use nfd2nfc_core::constants::HEARTBEAT_CHECK_INTERVAL;

use crate::daemon_controller;
use crate::tui::app::events::MouseState;
use crate::tui::component::{SharedState, TabComponent};
use crate::tui::tabs::{BrowserState, ConfigState, HomeState, LogsState, Tab};
use crate::tui::tick_timer::TickTimer;
use crate::tui::toast::{ToastLevel, ToastState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingWatcherOperation {
    Starting,
    Stopping,
    Restarting,
}

pub struct AsyncOperation {
    pub kind: PendingWatcherOperation,
    pub result_rx: Receiver<Result<(), String>>,
}

/// Main application state
pub struct App {
    pub running: bool,
    pub force_redraw: bool,
    pub current_tab: Tab,
    pub watcher_running: bool,
    pub home: HomeState,
    pub config: ConfigState,
    pub logs: LogsState,
    pub browser: BrowserState,
    pub toast: ToastState,
    pub async_operation: Option<AsyncOperation>,
    pub mouse_state: MouseState,
    heartbeat_timer: TickTimer,
}

impl App {
    pub fn new() -> Self {
        let watcher_running = daemon_controller::check_watcher_status();
        let (loaded_config, load_err) = load_config();
        let mut config = ConfigState::from_config(loaded_config);
        let mut toast = ToastState::new();
        if let Some(e) = load_err {
            config.has_changes = true;
            toast.push(e.user_message().to_string(), ToastLevel::Error);
        }

        Self {
            running: true,
            force_redraw: false,
            current_tab: Tab::Home,
            watcher_running,
            home: HomeState::default(),
            config,
            logs: LogsState::new(),
            browser: BrowserState::new(),
            toast,
            async_operation: None,
            mouse_state: MouseState::default(),
            heartbeat_timer: TickTimer::new(HEARTBEAT_CHECK_INTERVAL),
        }
    }

    pub fn shared_state(&self) -> SharedState {
        SharedState {
            watcher_running: self.watcher_running,
            async_op_pending: self.async_operation.is_some(),
            pending_operation: self.async_operation.as_ref().map(|op| op.kind),
            current_tab: self.current_tab,
        }
    }

    pub fn tick(&mut self) {
        self.toast.tick();

        // Check for async operation completion
        if let Some(ref op) = self.async_operation {
            if let Ok(result) = op.result_rx.try_recv() {
                match (&op.kind, result) {
                    (PendingWatcherOperation::Starting, Ok(())) => {
                        self.watcher_running = true;
                        self.show_toast("Watcher started".to_string(), false);
                    }
                    (PendingWatcherOperation::Starting, Err(e)) => {
                        self.show_toast(format!("Failed to start: {}", e), true);
                    }
                    (PendingWatcherOperation::Stopping, Ok(())) => {
                        self.watcher_running = false;
                        self.show_toast("Watcher stopped".to_string(), false);
                    }
                    (PendingWatcherOperation::Stopping, Err(e)) => {
                        self.show_toast(format!("Failed to stop: {}", e), true);
                    }
                    (PendingWatcherOperation::Restarting, Ok(())) => {
                        self.watcher_running = true;
                        self.show_toast("Watcher restarted".to_string(), false);
                    }
                    (PendingWatcherOperation::Restarting, Err(e)) => {
                        self.show_toast(format!("Failed to restart: {}", e), true);
                    }
                }
                self.async_operation = None;
            }
        }

        // Update watcher status only when no operation is pending, throttled to 1s interval
        if self.async_operation.is_none() && self.heartbeat_timer.ready() {
            self.watcher_running = daemon_controller::check_watcher_status();
        }

        // Tick all tab components
        let shared = self.shared_state();
        self.home.tick(&shared);
        self.config.tick(&shared);
        self.logs.tick(&shared);
        self.browser.tick(&shared);
    }

    pub fn show_toast(&mut self, message: String, is_error: bool) {
        let level = if is_error {
            ToastLevel::Error
        } else {
            ToastLevel::Success
        };
        self.toast.push(message, level);
    }

    pub fn start_async_operation(
        &mut self,
        operation: PendingWatcherOperation,
        rx: Receiver<Result<(), String>>,
    ) {
        self.async_operation = Some(AsyncOperation {
            kind: operation,
            result_rx: rx,
        });
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Home => Tab::Config,
            Tab::Config => Tab::Logs,
            Tab::Logs => Tab::Browser,
            Tab::Browser => Tab::Home,
        };
        self.force_redraw = true;
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Home => Tab::Browser,
            Tab::Config => Tab::Home,
            Tab::Logs => Tab::Config,
            Tab::Browser => Tab::Logs,
        };
        self.force_redraw = true;
    }

    pub fn select_tab(&mut self, tab: Tab) {
        self.current_tab = tab;
        self.force_redraw = true;
    }
}
