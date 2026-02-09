use crate::constants::HOME_DIR;
use std::path::{Path, PathBuf};

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        let mut home_path = HOME_DIR.clone();
        home_path.push(stripped);
        home_path
    } else {
        PathBuf::from(path)
    }
}

pub fn abbreviate_home(path_str: &str) -> String {
    let home = HOME_DIR.to_string_lossy();
    if let Some(rest) = path_str.strip_prefix(home.as_ref()) {
        format!("~{rest}")
    } else {
        path_str.to_string()
    }
}

pub fn abbreviate_home_path(path: &Path) -> String {
    abbreviate_home(&path.display().to_string())
}
