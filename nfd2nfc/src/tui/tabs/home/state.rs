use std::sync::mpsc::{self, Receiver};

use crate::tui::component::Action;
use crate::version::check_latest_version;

#[derive(Debug)]
pub struct HomeState {
    pub available_update: Option<String>,
    version_rx: Option<Receiver<Option<String>>>,
}

impl HomeState {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(check_latest_version());
        });
        Self {
            available_update: None,
            version_rx: Some(rx),
        }
    }

    pub fn tick_version_check(&mut self) -> Option<Action> {
        if let Some(ref rx) = self.version_rx
            && let Ok(result) = rx.try_recv()
        {
            self.available_update = result;
            self.version_rx = None;
            if let Some(ver) = self.available_update.as_deref() {
                return Some(Action::ShowToast {
                    message: format!("nfd2nfc v{ver} available"),
                    is_error: false,
                });
            }
        }
        None
    }
}
