use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::app::events::MouseState;
use crate::tui::app::state::PendingWatcherOperation;
use crate::tui::tabs::Tab;

/// Read-only shared state passed to tab components (Copy to avoid borrow conflicts)
#[derive(Debug, Clone, Copy)]
pub struct SharedState {
    pub watcher_running: bool,
    pub async_op_pending: bool,
    pub pending_operation: Option<PendingWatcherOperation>,
    pub current_tab: Tab,
}

/// Scroll direction for mouse wheel events
pub enum ScrollDirection {
    Up,
    Down,
}

/// Actions that a tab component can return to request App-level operations
pub enum Action {
    Quit,
    NextTab,
    PreviousTab,
    SelectTab(Tab),
    ShowToast { message: String, is_error: bool },
    StartWatcher,
    StopWatcher,
    RestartWatcher,
    ConfigSaved,
    ReloadConfig,
    Consumed,
}

/// Trait that all tab components must implement
pub trait TabComponent {
    fn render(&mut self, f: &mut Frame, area: Rect, shared: &SharedState, mouse: &mut MouseState);
    fn handle_key(&mut self, key: KeyCode, shared: &SharedState) -> Option<Action>;

    fn handle_scroll(&mut self, _direction: ScrollDirection) -> Option<Action> {
        None
    }

    fn handle_mouse_click(&mut self, _x: u16, _y: u16) -> Option<Action> {
        None
    }

    fn handle_double_click(&mut self, _x: u16, _y: u16) -> Option<Action> {
        None
    }

    fn tick(&mut self, _shared: &SharedState) -> Option<Action> {
        None
    }
}
