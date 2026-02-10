use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use ratatui::style::Color;
use ratatui::widgets::ListState;

use nfd2nfc_core::constants::HOME_DIR;
use nfd2nfc_core::normalizer::get_actual_file_name;
use nfd2nfc_core::{is_nfc, is_nfd};
use unicode_normalization::UnicodeNormalization;

use crate::tui::component::{next_index, prev_index};
use crate::tui::tick_timer::TickTimer;

const AUTO_REFRESH_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum UnicodeForm {
    NFC,
    NFD,
    ASCII,
    Mixed,
}

impl UnicodeForm {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnicodeForm::NFC => "NFC",
            UnicodeForm::NFD => "NFD",
            UnicodeForm::ASCII => "",
            UnicodeForm::Mixed => "Mixed",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            UnicodeForm::NFC => Color::Green,
            UnicodeForm::NFD => Color::Yellow,
            UnicodeForm::ASCII => Color::White,
            UnicodeForm::Mixed => Color::Magenta,
        }
    }
}

/// Classification of the selected entry (used for bottom menu display and key behavior)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionKind {
    /// ../
    Parent,
    /// Directory with NFD/NFC name
    DirUnicode,
    /// Directory with ASCII/Mixed name
    DirAscii,
    /// File with NFD name
    FileNFD,
    /// File with NFC name
    FileNFC,
    /// File with ASCII/Mixed name — no conversion possible
    FileAscii,
    /// No selection
    None,
}

impl SelectionKind {
    pub fn is_dir(&self) -> bool {
        matches!(self, Self::DirUnicode | Self::DirAscii)
    }

    pub fn is_inactive(&self) -> bool {
        matches!(self, Self::Parent | Self::FileAscii | Self::None)
    }
}

#[derive(Debug, Clone)]
pub struct BrowserEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_parent: bool,
    pub form: UnicodeForm,
}

pub struct DirBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<BrowserEntry>,
    pub list_state: ListState,
    pub show_hidden: bool,
    pub render_offset: usize,
    auto_refresh_timer: TickTimer,
}

impl std::fmt::Debug for DirBrowser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirBrowser")
            .field("current_dir", &self.current_dir)
            .field("entries_count", &self.entries.len())
            .field("show_hidden", &self.show_hidden)
            .finish()
    }
}

impl DirBrowser {
    pub fn new() -> Self {
        let mut browser = Self {
            current_dir: HOME_DIR.clone(),
            entries: Vec::new(),
            list_state: ListState::default(),
            show_hidden: false,
            render_offset: 0,
            auto_refresh_timer: TickTimer::new(AUTO_REFRESH_INTERVAL),
        };
        browser.refresh();
        browser
    }

    pub fn tick(&mut self, active: bool) {
        if active && self.auto_refresh_timer.ready() {
            self.refresh();
        }
    }

    pub fn refresh(&mut self) {
        // Sync current_dir with actual disk path, or fallback to ancestor if invalid
        match self.current_dir.canonicalize() {
            Ok(canonical) => {
                self.current_dir = canonical;
            }
            Err(_) => {
                let mut fallback = self.current_dir.clone();
                loop {
                    if let Some(parent) = fallback.parent() {
                        fallback = parent.to_path_buf();
                        if fallback.is_dir() {
                            break;
                        }
                    } else {
                        fallback = HOME_DIR.clone();
                        break;
                    }
                }
                self.current_dir = fallback;
                self.list_state.select(Some(0));
                self.render_offset = 0;
            }
        }

        let prev_selected_name = self
            .selected_entry()
            .map(|e| e.name.nfc().collect::<String>());

        self.entries.clear();

        let read_result = fs::read_dir(&self.current_dir);
        if let Ok(entries) = read_result {
            let show_hidden = self.show_hidden;
            let mut items: Vec<BrowserEntry> = entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let path = entry.path();
                    let is_dir = path.is_dir();

                    // Get the actual name from disk
                    let name = if let Ok(actual) = get_actual_file_name(&path) {
                        actual
                    } else {
                        path.file_name()?.to_string_lossy().to_string()
                    };

                    // Filter hidden files (dotfiles)
                    if !show_hidden && name.starts_with('.') {
                        return None;
                    }

                    let form = detect_unicode_form(&name);

                    Some(BrowserEntry {
                        path,
                        name,
                        is_dir,
                        is_parent: false,
                        form,
                    })
                })
                .collect();

            // Sort: directories first, then by name
            items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });

            self.entries = items;
        }

        // Insert parent entry at index 0
        if let Some(parent) = self.current_dir.parent() {
            self.entries.insert(
                0,
                BrowserEntry {
                    path: parent.to_path_buf(),
                    name: "..".to_string(),
                    is_dir: true,
                    is_parent: true,
                    form: UnicodeForm::ASCII,
                },
            );
        }

        // Restore selection by NFC-normalized filename, or clamp index as fallback
        if self.entries.is_empty() {
            self.list_state.select(None);
        } else if let Some(prev_name) = &prev_selected_name {
            if let Some(new_idx) = self
                .entries
                .iter()
                .position(|e| e.name.nfc().collect::<String>() == *prev_name)
            {
                self.list_state.select(Some(new_idx));
            } else if let Some(idx) = self.list_state.selected() {
                self.list_state
                    .select(Some(idx.min(self.entries.len() - 1)));
            }
        } else if self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        }
    }

    pub fn selected_entry(&self) -> Option<&BrowserEntry> {
        self.list_state
            .selected()
            .and_then(|idx| self.entries.get(idx))
    }

    pub fn selection_kind(&self) -> SelectionKind {
        match self.selected_entry() {
            None => SelectionKind::None,
            Some(e) if e.is_parent => SelectionKind::Parent,
            Some(e) if e.is_dir => match e.form {
                UnicodeForm::NFD | UnicodeForm::NFC => SelectionKind::DirUnicode,
                _ => SelectionKind::DirAscii,
            },
            Some(e) => match e.form {
                UnicodeForm::NFD => SelectionKind::FileNFD,
                UnicodeForm::NFC => SelectionKind::FileNFC,
                _ => SelectionKind::FileAscii,
            },
        }
    }

    pub fn select_next(&mut self) {
        if let Some(i) = next_index(self.list_state.selected(), self.entries.len()) {
            self.list_state.select(Some(i));
        }
    }

    pub fn select_previous(&mut self) {
        if let Some(i) = prev_index(self.list_state.selected(), self.entries.len()) {
            self.list_state.select(Some(i));
        }
    }

    pub fn dir_indices(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_dir)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn select_next_dir(&mut self) {
        let dirs = self.dir_indices();
        if dirs.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        if let Some(&next) = dirs.iter().find(|&&i| i > current) {
            self.list_state.select(Some(next));
        }
    }

    pub fn select_previous_dir(&mut self) {
        let dirs = self.dir_indices();
        if dirs.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        if let Some(&prev) = dirs.iter().rev().find(|&&i| i < current) {
            self.list_state.select(Some(prev));
        }
    }

    pub fn try_enter_selected(&mut self) {
        let path = self
            .selected_entry()
            .filter(|e| e.is_dir && !e.is_parent)
            .map(|e| e.path.clone());
        if let Some(path) = path {
            self.enter_directory(&path);
        }
    }

    pub fn enter_directory(&mut self, path: &std::path::Path) {
        if path.is_dir() {
            self.current_dir = path.to_path_buf();
            self.list_state.select(Some(0));
            self.render_offset = 0;
            self.refresh();
        }
    }

    pub fn go_parent(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            let old_dir = self.current_dir.clone();
            self.current_dir = parent.to_path_buf();
            self.render_offset = 0;
            self.refresh();

            // Try to select the directory we came from
            if let Some(idx) = self.entries.iter().position(|e| e.path == old_dir) {
                self.list_state.select(Some(idx));
            }
        }
    }

    pub fn toggle_hidden(&mut self) {
        let selected_path = self.selected_entry().map(|e| e.path.clone());
        self.show_hidden = !self.show_hidden;
        self.refresh();
        if let Some(path) = selected_path {
            if let Some(idx) = self.entries.iter().position(|e| e.path == path) {
                self.list_state.select(Some(idx));
            }
        }
    }
}

pub fn detect_unicode_form(name: &str) -> UnicodeForm {
    let is_ascii = name.is_ascii();
    if is_ascii {
        return UnicodeForm::ASCII;
    }

    let nfc = is_nfc(name);
    let nfd = is_nfd(name);

    match (nfc, nfd) {
        (true, false) => UnicodeForm::NFC,
        (false, true) => UnicodeForm::NFD,
        (true, true) => UnicodeForm::ASCII, // Both true means it's ASCII-compatible
        (false, false) => UnicodeForm::Mixed,
    }
}
