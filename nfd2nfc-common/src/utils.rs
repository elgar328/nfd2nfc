use crate::constants::HOME_DIR;
use std::path::PathBuf;

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        let mut home_path = HOME_DIR.clone();
        home_path.push(stripped);
        home_path
    } else {
        PathBuf::from(path)
    }
}
