use crossterm::event::KeyCode;

use crate::tui::app::render::content_area;
use crate::tui::component::{Action, ScrollDirection, SharedState};
use crate::tui::tabs::logs::state::LogsState;

pub fn handle_key(state: &mut LogsState, key: KeyCode, _shared: &SharedState) -> Option<Action> {
    let visible_height = state.visible_height;

    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            state.scroll_up(1);
            None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.scroll_down(1);
            None
        }
        KeyCode::PageUp | KeyCode::Char('u') => {
            state.scroll_up(visible_height.saturating_sub(2));
            None
        }
        KeyCode::PageDown | KeyCode::Char('d') => {
            state.scroll_down(visible_height.saturating_sub(2));
            None
        }
        KeyCode::Char('t') => {
            state.go_to_top();
            None
        }
        KeyCode::Char('b') => {
            state.go_to_bottom();
            None
        }
        _ => None,
    }
}

pub fn handle_scroll(state: &mut LogsState, direction: ScrollDirection) -> Option<Action> {
    match direction {
        ScrollDirection::Up => state.scroll_up(3),
        ScrollDirection::Down => state.scroll_down(3),
    }
    None
}

pub fn handle_mouse_click(state: &mut LogsState, _x: u16, y: u16) -> Option<Action> {
    let ca = content_area();
    let inner_y = ca.y + 1;
    let inner_height = ca.height.saturating_sub(2);

    if y >= inner_y && y < inner_y + inner_height {
        let clicked_line = (y - inner_y) as usize;
        let target_offset = state
            .scroll_offset
            .saturating_add(clicked_line)
            .saturating_sub(state.visible_height / 2);
        let max_offset = state
            .line_cache
            .total_lines()
            .saturating_sub(state.visible_height);
        state.scroll_offset = target_offset.min(max_offset);
        state.auto_scroll = false;
    }
    None
}
