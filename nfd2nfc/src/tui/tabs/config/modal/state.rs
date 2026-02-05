use crate::tui::dir_browser::DirBrowser;
use nfd2nfc_core::config::{PathAction, PathMode};

#[derive(Debug)]
pub struct AddModalState {
    pub show: bool,
    pub browser: DirBrowser,
    pub action: PathAction,
    pub mode: PathMode,
    pub path_box_height: u16,
}

impl AddModalState {
    pub fn new() -> Self {
        Self {
            show: false,
            browser: DirBrowser::new(),
            action: PathAction::Watch,
            mode: PathMode::Recursive,
            path_box_height: 3,
        }
    }

    pub fn tick(&mut self) {
        if self.show {
            self.browser.tick(true);
        }
    }
}
