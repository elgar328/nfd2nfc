use crossterm::event::KeyCode;

use crate::tui::component::{Action, SharedState};
use crate::tui::tabs::home::state::HomeState;

pub fn handle_key(_state: &mut HomeState, key: KeyCode, shared: &SharedState) -> Option<Action> {
    // Ignore keys while operation is in progress
    if shared.async_op_pending {
        return None;
    }

    match key {
        KeyCode::Char('s') => {
            if shared.watcher_running {
                Some(Action::StopWatcher)
            } else {
                Some(Action::StartWatcher)
            }
        }
        KeyCode::Char('r') => {
            if shared.watcher_running {
                Some(Action::RestartWatcher)
            } else {
                None
            }
        }
        _ => None,
    }
}
