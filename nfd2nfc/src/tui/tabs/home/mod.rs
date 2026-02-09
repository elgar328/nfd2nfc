pub mod events;
pub mod render;
pub mod state;

pub use state::HomeState;

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::tui::app::events::MouseState;
use crate::tui::component::{Action, SharedState, TabComponent};

impl TabComponent for HomeState {
    fn render(&mut self, f: &mut Frame, area: Rect, shared: &SharedState, mouse: &mut MouseState) {
        render::render(self, f, area, shared, mouse);
    }

    fn handle_key(&mut self, key: KeyCode, shared: &SharedState) -> Option<Action> {
        events::handle_key(self, key, shared)
    }

    fn tick(&mut self, _shared: &SharedState) {
        self.tick_version_check();
    }
}
