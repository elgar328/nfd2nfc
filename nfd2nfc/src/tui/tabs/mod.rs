pub mod browser;
pub mod config;
pub mod home;
pub mod logs;

pub use browser::BrowserState;
pub use config::ConfigState;
use crossterm::event::KeyCode;
pub use home::HomeState;
pub use logs::LogsState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter)]
pub enum Tab {
    Home = 0,
    Config = 1,
    Logs = 2,
    Browser = 3,
}

impl Tab {
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Home => "Home",
            Tab::Config => "Config",
            Tab::Logs => "Logs",
            Tab::Browser => "Browser",
        }
    }

    pub fn superscript(&self) -> &'static str {
        match self {
            Tab::Home => "¹",
            Tab::Config => "²",
            Tab::Logs => "³",
            Tab::Browser => "⁴",
        }
    }

    pub fn index(&self) -> usize {
        *self as usize
    }

    pub fn key(&self) -> KeyCode {
        match self {
            Tab::Home => KeyCode::Char('1'),
            Tab::Config => KeyCode::Char('2'),
            Tab::Logs => KeyCode::Char('3'),
            Tab::Browser => KeyCode::Char('4'),
        }
    }

    pub fn from_key(key: KeyCode) -> Option<Tab> {
        match key {
            KeyCode::Char('1') => Some(Tab::Home),
            KeyCode::Char('2') => Some(Tab::Config),
            KeyCode::Char('3') => Some(Tab::Logs),
            KeyCode::Char('4') => Some(Tab::Browser),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Tab::Home => Tab::Config,
            Tab::Config => Tab::Logs,
            Tab::Logs => Tab::Browser,
            Tab::Browser => Tab::Home,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Tab::Home => Tab::Browser,
            Tab::Config => Tab::Home,
            Tab::Logs => Tab::Config,
            Tab::Browser => Tab::Logs,
        }
    }
}
