use nfd2nfc_core::normalizer::{normalize_directory, normalize_single_file, NormalizationTarget};

use crate::tui::dir_browser::{DirBrowser, UnicodeForm};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserAction {
    Convert, // NFD -> NFC
    Reverse, // NFC -> NFD
}

impl BrowserAction {
    pub fn toggle(&self) -> Self {
        match self {
            BrowserAction::Convert => BrowserAction::Reverse,
            BrowserAction::Reverse => BrowserAction::Convert,
        }
    }

    pub fn to_target(self) -> NormalizationTarget {
        match self {
            BrowserAction::Convert => NormalizationTarget::NFC,
            BrowserAction::Reverse => NormalizationTarget::NFD,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserMode {
    Recursive,
    Children,
    NameOnly,
}

impl BrowserMode {
    pub fn cycle(&self) -> Self {
        match self {
            BrowserMode::NameOnly => BrowserMode::Children,
            BrowserMode::Children => BrowserMode::Recursive,
            BrowserMode::Recursive => BrowserMode::NameOnly,
        }
    }

    /// For ASCII folders: skip NameOnly (Children ↔ Recursive)
    pub fn cycle_skip_name_only(&self) -> Self {
        match self {
            BrowserMode::NameOnly => BrowserMode::Children,
            BrowserMode::Children => BrowserMode::Recursive,
            BrowserMode::Recursive => BrowserMode::Children,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BrowserMode::Recursive => "Recursive",
            BrowserMode::Children => "Children only",
            BrowserMode::NameOnly => "Name only",
        }
    }
}

pub struct BrowserState {
    pub dir_browser: DirBrowser,
    pub action: BrowserAction,
    pub mode: BrowserMode,
    pub path_height: u16,
}

impl std::fmt::Debug for BrowserState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserState")
            .field("dir_browser", &self.dir_browser)
            .field("action", &self.action)
            .field("mode", &self.mode)
            .finish()
    }
}

impl BrowserState {
    pub fn new() -> Self {
        Self {
            dir_browser: DirBrowser::new(),
            action: BrowserAction::Convert,
            mode: BrowserMode::NameOnly,
            path_height: 3,
        }
    }

    pub fn toggle_action(&mut self) {
        self.action = self.action.toggle();
    }

    pub fn cycle_mode(&mut self) {
        self.mode = self.mode.cycle();
    }

    /// Auto-switch from NameOnly to Children when an ASCII/Mixed folder is selected.
    pub fn auto_adjust_mode(&mut self) {
        if let Some(entry) = self.dir_browser.selected_entry() {
            if entry.is_dir
                && self.mode == BrowserMode::NameOnly
                && matches!(entry.form, UnicodeForm::ASCII | UnicodeForm::Mixed)
            {
                self.mode = BrowserMode::Children;
            }
        }
    }

    pub fn convert_selected(&mut self) -> Result<(), String> {
        let entry = match self.dir_browser.selected_entry().cloned() {
            Some(e) => e,
            None => return Err("No item selected".to_string()),
        };

        let target = self.action.to_target();
        let path = &entry.path;

        // Calculate the expected new path after conversion
        let new_path = if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            path.with_file_name(target.convert(name))
        } else {
            path.clone()
        };

        let result = if self.mode == BrowserMode::NameOnly || !path.is_dir() {
            normalize_single_file(path, target)
        } else {
            let recursive = self.mode == BrowserMode::Recursive;
            normalize_directory(path, recursive, target)
        };

        result.map_err(|e| e.to_string())?;

        // Refresh after conversion
        self.dir_browser.refresh();

        // Try to select the converted path
        if let Some(idx) = self
            .dir_browser
            .entries
            .iter()
            .position(|e| e.path == new_path)
        {
            self.dir_browser.list_state.select(Some(idx));
        }

        Ok(())
    }
}
