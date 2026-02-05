use crossterm::event::KeyCode;

use crate::tui::app::render::content_area;
use crate::tui::component::{Action, ScrollDirection, SharedState};
use crate::tui::tabs::config::modal::events as modal_events;
use crate::tui::tabs::config::modal::events::{
    handle_modal_double_click, handle_modal_mouse_click, handle_modal_scroll,
};
use crate::tui::tabs::config::state::ConfigState;

pub fn handle_key(state: &mut ConfigState, key: KeyCode, _shared: &SharedState) -> Option<Action> {
    if state.modal.show {
        let (action, add_result) = modal_events::handle_modal_key(&mut state.modal, key);
        if let Some(result) = add_result {
            state.add_path(result.path, result.action, result.mode);
        }
        return action.or(Some(Action::Consumed));
    }

    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            state.select_previous();
            None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.select_next();
            None
        }
        KeyCode::Char('a') => {
            state.modal.show = true;
            state.modal.browser.refresh();
            None
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            state.delete_selected();
            None
        }
        KeyCode::Char('t') => {
            state.toggle_action();
            None
        }
        KeyCode::Char('m') => {
            state.toggle_mode();
            None
        }
        KeyCode::Char('o') => {
            state.sort_paths();
            None
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            state.move_up();
            None
        }
        KeyCode::Char('-') | KeyCode::Char('_') => {
            state.move_down();
            None
        }
        KeyCode::Char('s') => {
            if state.has_changes {
                match state.save() {
                    Ok(_) => Some(Action::ConfigSaved),
                    Err(e) => Some(Action::ShowToast {
                        message: e,
                        is_error: true,
                    }),
                }
            } else {
                None
            }
        }
        KeyCode::Esc => {
            if state.has_changes {
                Some(Action::ReloadConfig)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn handle_scroll(state: &mut ConfigState, direction: ScrollDirection) -> Option<Action> {
    if state.modal.show {
        return handle_modal_scroll(&mut state.modal, direction);
    }
    match direction {
        ScrollDirection::Up => state.select_previous(),
        ScrollDirection::Down => state.select_next(),
    }
    None
}

pub fn handle_mouse_click(state: &mut ConfigState, x: u16, y: u16) -> Option<Action> {
    if state.modal.show {
        return handle_modal_mouse_click(&mut state.modal, x, y);
    }
    handle_table_mouse_click(state, y);
    None
}

fn handle_table_mouse_click(state: &mut ConfigState, y: u16) {
    let ca = content_area();
    let inner_y = ca.y + 1;
    let table_data_start_y = inner_y + 2;
    let table_end_y = ca.y + ca.height - 1;

    if y >= table_data_start_y && y < table_end_y {
        let clicked_index = (y - table_data_start_y) as usize;
        if clicked_index < state.config.paths.len() {
            state.table_state.select(Some(clicked_index));
        }
    }
}

pub fn handle_double_click(state: &mut ConfigState, x: u16, y: u16) -> Option<Action> {
    if state.modal.show {
        return handle_modal_double_click(&mut state.modal, x, y);
    }
    None
}
