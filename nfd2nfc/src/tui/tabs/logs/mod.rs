pub mod events;
pub mod render;
pub mod state;

pub use state::LogsState;

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::app::events::MouseState;
use crate::tui::component::{Action, ScrollDirection, SharedState, TabComponent};

impl TabComponent for LogsState {
    fn render(&mut self, f: &mut Frame, area: Rect, shared: &SharedState, mouse: &mut MouseState) {
        render::render(self, f, area, shared, mouse);
    }

    fn handle_key(&mut self, key: KeyCode, shared: &SharedState) -> Option<Action> {
        events::handle_key(self, key, shared)
    }

    fn handle_scroll(&mut self, direction: ScrollDirection) -> Option<Action> {
        events::handle_scroll(self, direction)
    }

    fn handle_mouse_click(&mut self, x: u16, y: u16) -> Option<Action> {
        events::handle_mouse_click(self, x, y)
    }

    fn tick(&mut self, _shared: &SharedState) {
        self.process_events();
    }
}
