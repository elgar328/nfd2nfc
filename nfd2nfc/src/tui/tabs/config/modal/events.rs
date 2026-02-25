use std::path::PathBuf;

use crossterm::event::KeyCode;

use crate::tui::component::{Action, ScrollDirection};
use crate::tui::dir_browser::SelectionKind;
use crate::tui::tabs::config::modal::state::AddModalState;
use nfd2nfc_core::config::{PathAction, PathMode};

/// Result from modal key handling that needs to be applied to ConfigState by the caller
pub struct ModalAddResult {
    pub path: PathBuf,
    pub action: PathAction,
    pub mode: PathMode,
}

/// Returns (Option<Action>, Option<ModalAddResult>)
/// The caller is responsible for applying ModalAddResult to ConfigState.
pub fn handle_modal_key(
    modal: &mut AddModalState,
    key: KeyCode,
) -> (Option<Action>, Option<ModalAddResult>) {
    match key {
        KeyCode::Esc => {
            modal.show = false;
            (None, None)
        }
        KeyCode::Up | KeyCode::Char('k') => {
            modal.browser.select_previous_dir();
            (None, None)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            modal.browser.select_next_dir();
            (None, None)
        }
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
            modal.browser.go_parent();
            (None, None)
        }
        KeyCode::Right | KeyCode::Char('l') => {
            modal.browser.try_enter_selected();
            (None, None)
        }
        KeyCode::Enter => {
            if modal.browser.selection_kind() == SelectionKind::Parent {
                return (None, None);
            }
            let path = modal
                .browser
                .selected_entry()
                .map(|e| e.path.clone())
                .unwrap_or_else(|| modal.browser.current_dir.clone());
            let action = modal.action;
            let mode = modal.mode;
            modal.show = false;
            (
                Some(Action::ShowToast {
                    message: "Path added".to_string(),
                    is_error: false,
                }),
                Some(ModalAddResult { path, action, mode }),
            )
        }
        KeyCode::Char('t') => {
            if modal.browser.selection_kind() == SelectionKind::Parent {
                return (None, None);
            }
            modal.action = modal.action.toggle();
            if modal.action == PathAction::Ignore {
                modal.mode = PathMode::Recursive;
            }
            (None, None)
        }
        KeyCode::Char('m') => {
            if modal.browser.selection_kind() == SelectionKind::Parent {
                return (None, None);
            }
            if modal.action != PathAction::Ignore {
                modal.mode = modal.mode.toggle();
            }
            (None, None)
        }
        KeyCode::Char('.') => {
            modal.browser.toggle_hidden();
            (None, None)
        }
        _ => (None, None),
    }
}

pub fn handle_modal_scroll(
    modal: &mut AddModalState,
    direction: ScrollDirection,
) -> Option<Action> {
    match direction {
        ScrollDirection::Up => modal.browser.select_previous_dir(),
        ScrollDirection::Down => modal.browser.select_next_dir(),
    }
    None
}

/// Resolve a click at the given y coordinate to a dir entry index in the modal browser.
fn resolve_click_index(modal: &AddModalState, y: u16) -> Option<usize> {
    let full_area = ratatui::layout::Rect::new(
        0,
        0,
        crossterm::terminal::size().map(|(w, _)| w).unwrap_or(80),
        crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24),
    );

    let modal_rect = super::modal_area(full_area);
    let modal_y = modal_rect.y;

    let inner_y = modal_y + 1;
    let list_block_y = inner_y + modal.path_box_height;
    let list_start_y = list_block_y + 1;
    let list_end_y = modal_y + modal_rect.height - 1 - 2 - 1;

    if y < list_start_y || y >= list_end_y {
        return None;
    }

    let dir_entries = modal.browser.dir_indices();

    let clicked_list_index = (y - list_start_y) as usize + modal.browser.render_offset;
    dir_entries.get(clicked_list_index).copied()
}

pub fn handle_modal_mouse_click(modal: &mut AddModalState, _x: u16, y: u16) -> Option<Action> {
    if let Some(index) = resolve_click_index(modal, y) {
        modal.browser.list_state.select(Some(index));
    }
    None
}

pub fn handle_modal_double_click(modal: &mut AddModalState, _x: u16, y: u16) -> Option<Action> {
    if let Some(index) = resolve_click_index(modal, y)
        && let Some(entry) = modal.browser.entries.get(index)
    {
        if entry.is_parent {
            modal.browser.go_parent();
        } else if entry.is_dir {
            let path = entry.path.clone();
            modal.browser.enter_directory(&path);
        }
    }
    None
}
