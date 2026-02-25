use std::path::PathBuf;
use std::time::Duration;

use ratatui::widgets::TableState;

use nfd2nfc_core::config::{Config, PathAction, PathEntry, PathMode, load_config};
use nfd2nfc_core::constants::CONFIG_PATH;

use crate::tui::component::{SharedState, next_index, prev_index};
use crate::tui::tabs::Tab;
use crate::tui::tabs::config::modal::state::AddModalState;
use crate::tui::tick_timer::TickTimer;

const STATUS_REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const CONFIG_RELOAD_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub struct ConfigState {
    pub config: Config,
    pub table_state: TableState,
    pub modal: AddModalState,
    pub has_changes: bool,
    status_refresh_timer: TickTimer,
    config_reload_timer: TickTimer,
}

impl ConfigState {
    pub fn from_config(config: Config) -> Self {
        let mut state = Self {
            config,
            table_state: TableState::default(),
            modal: AddModalState::new(),
            has_changes: false,
            status_refresh_timer: TickTimer::new(STATUS_REFRESH_INTERVAL),
            config_reload_timer: TickTimer::new(CONFIG_RELOAD_INTERVAL),
        };

        if !state.config.paths.is_empty() {
            state.table_state.select(Some(0));
        }

        state
    }

    fn mark_changed(&mut self) {
        self.has_changes = true;
        self.config.refresh_statuses();
    }

    pub fn select_next(&mut self) {
        if let Some(i) = next_index(self.table_state.selected(), self.config.paths.len()) {
            self.table_state.select(Some(i));
        }
    }

    pub fn select_previous(&mut self) {
        if let Some(i) = prev_index(self.table_state.selected(), self.config.paths.len()) {
            self.table_state.select(Some(i));
        }
    }

    fn move_item(&mut self, offset: isize) {
        if let Some(i) = self.table_state.selected()
            && let Some(new_i) = i.checked_add_signed(offset)
            && new_i < self.config.paths.len()
        {
            self.config.paths.swap(i, new_i);
            self.table_state.select(Some(new_i));
            self.mark_changed();
        }
    }

    pub fn move_up(&mut self) {
        self.move_item(-1);
    }
    pub fn move_down(&mut self) {
        self.move_item(1);
    }

    pub fn toggle_action(&mut self) {
        if let Some(i) = self.table_state.selected() {
            self.config.paths[i].action = self.config.paths[i].action.toggle();
            if self.config.paths[i].action == PathAction::Ignore {
                self.config.paths[i].mode = PathMode::Recursive;
            }
            self.mark_changed();
        }
    }

    pub fn toggle_mode(&mut self) {
        if let Some(i) = self.table_state.selected() {
            if self.config.paths[i].action == PathAction::Ignore {
                return;
            }
            self.config.paths[i].mode = self.config.paths[i].mode.toggle();
            self.mark_changed();
        }
    }

    pub fn delete_selected(&mut self) {
        if let Some(i) = self.table_state.selected() {
            self.config.paths.remove(i);
            if self.config.paths.is_empty() {
                self.table_state.select(None);
            } else if i >= self.config.paths.len() {
                self.table_state.select(Some(self.config.paths.len() - 1));
            }
            self.mark_changed();
        }
    }

    pub fn add_path(&mut self, path: PathBuf, action: PathAction, mode: PathMode) {
        self.config.paths.push(PathEntry::new(path, action, mode));
        self.mark_changed();
        self.table_state.select(Some(self.config.paths.len() - 1));
    }

    pub fn reload(&mut self) {
        let selected = self.table_state.selected();
        let (config, _) = load_config();
        *self = Self::from_config(config);
        if let Some(i) = selected
            && i < self.config.paths.len()
        {
            self.table_state.select(Some(i));
        }
    }

    pub fn sort_paths(&mut self) {
        if self.config.paths.len() <= 1 {
            return;
        }
        let selected_raw = self
            .table_state
            .selected()
            .map(|i| self.config.paths[i].raw.clone());
        self.config.paths.sort_by(|a, b| a.raw.cmp(&b.raw));
        if let Some(raw) = selected_raw
            && let Some(new_idx) = self.config.paths.iter().position(|p| p.raw == raw)
        {
            self.table_state.select(Some(new_idx));
        }
        self.mark_changed();
    }

    pub fn poll(&mut self, shared: &SharedState) {
        if shared.current_tab == Tab::Config {
            self.modal.tick();
            if !self.modal.show {
                if self.status_refresh_timer.ready() {
                    self.config.refresh_statuses();
                }
                if !self.has_changes && self.config_reload_timer.ready() {
                    self.reload();
                }
            }
        }
    }

    pub fn save(&mut self) -> Result<(), String> {
        self.config
            .save_to_file(&CONFIG_PATH)
            .map_err(|e| format!("Failed to save config: {}", e))?;
        self.has_changes = false;
        Ok(())
    }
}
