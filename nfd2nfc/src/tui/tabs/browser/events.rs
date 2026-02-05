use crossterm::event::KeyCode;

use crate::tui::component::{Action, ScrollDirection, SharedState};
use crate::tui::dir_browser::{SelectionKind, UnicodeForm};
use crate::tui::tabs::browser::render::browser_list_y_range;
use crate::tui::tabs::browser::state::{BrowserAction, BrowserMode, BrowserState};

pub fn handle_key(state: &mut BrowserState, key: KeyCode, _shared: &SharedState) -> Option<Action> {
    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            state.dir_browser.select_previous();
            None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.dir_browser.select_next();
            None
        }
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
            state.dir_browser.go_parent();
            None
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if let Some(entry) = state.dir_browser.effective_selected_entry() {
                if entry.is_dir && !entry.is_parent {
                    state.dir_browser.enter_directory(&entry.path);
                }
            }
            None
        }
        KeyCode::Enter => {
            let kind = state.dir_browser.selection_kind();
            if matches!(
                kind,
                SelectionKind::Parent | SelectionKind::FileAscii | SelectionKind::None
            ) {
                return None;
            }

            // Auto-determine action for files or directories in NameOnly mode
            if let Some(entry) = state.dir_browser.effective_selected_entry() {
                let needs_auto_action = !entry.is_dir || state.mode == BrowserMode::NameOnly;

                if needs_auto_action {
                    match entry.form {
                        UnicodeForm::NFD => {
                            state.action = BrowserAction::Convert;
                        }
                        UnicodeForm::NFC => {
                            state.action = BrowserAction::Reverse;
                        }
                        UnicodeForm::ASCII | UnicodeForm::Mixed => {
                            return None;
                        }
                    }
                }
            }
            match state.convert_selected() {
                Ok(_) => Some(Action::ShowToast {
                    message: "Conversion completed".to_string(),
                    is_error: false,
                }),
                Err(e) => Some(Action::ShowToast {
                    message: format!("Conversion failed: {}", e),
                    is_error: true,
                }),
            }
        }
        KeyCode::Char('t') => {
            let kind = state.dir_browser.selection_kind();
            if kind.is_dir() {
                let is_name_only_with_unicode_name =
                    kind == SelectionKind::DirUnicode && state.mode == BrowserMode::NameOnly;
                if !is_name_only_with_unicode_name {
                    state.toggle_action();
                }
            }
            None
        }
        KeyCode::Char('m') => {
            let kind = state.dir_browser.selection_kind();
            if kind.is_dir() {
                if kind == SelectionKind::DirAscii {
                    state.mode = state.mode.cycle_skip_name_only();
                } else {
                    state.cycle_mode();
                }
            }
            None
        }
        KeyCode::Char('.') => {
            state.dir_browser.toggle_hidden();
            None
        }
        _ => None,
    }
}

pub fn handle_scroll(state: &mut BrowserState, direction: ScrollDirection) -> Option<Action> {
    match direction {
        ScrollDirection::Up => state.dir_browser.select_previous(),
        ScrollDirection::Down => state.dir_browser.select_next(),
    }
    None
}

pub fn handle_mouse_click(state: &mut BrowserState, _x: u16, y: u16) -> Option<Action> {
    let (list_start_y, list_end_y) = browser_list_y_range(state.path_height);

    if y >= list_start_y && y < list_end_y {
        let visible_index = (y - list_start_y) as usize;
        let scroll_offset = state.dir_browser.render_offset;
        let clicked_index = visible_index + scroll_offset;

        if clicked_index < state.dir_browser.entries.len() {
            state.dir_browser.list_state.select(Some(clicked_index));
        }
    }
    None
}

pub fn handle_double_click(state: &mut BrowserState, _x: u16, y: u16) -> Option<Action> {
    let (list_start_y, list_end_y) = browser_list_y_range(state.path_height);

    if y >= list_start_y && y < list_end_y {
        let visible_index = (y - list_start_y) as usize;
        let scroll_offset = state.dir_browser.render_offset;
        let clicked_index = visible_index + scroll_offset;

        if clicked_index < state.dir_browser.entries.len() {
            let entry = &state.dir_browser.entries[clicked_index];
            if entry.is_parent {
                state.dir_browser.go_parent();
            } else if entry.is_dir {
                let path = entry.path.clone();
                state.dir_browser.enter_directory(&path);
            }
        }
    }
    None
}
