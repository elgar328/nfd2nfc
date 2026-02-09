use std::sync::mpsc::{self, Receiver};

/// Check Homebrew for a newer version of nfd2nfc.
/// Returns `Some(latest_version)` if a newer version exists, `None` otherwise.
fn check_brew_update() -> Option<String> {
    let output = std::process::Command::new("brew")
        .args(["info", "--json=v2", "nfd2nfc"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let latest = json
        .get("formulae")?
        .get(0)?
        .get("versions")?
        .get("stable")?
        .as_str()?;

    let current = env!("CARGO_PKG_VERSION").trim_end_matches("-dev");

    if is_newer(latest, current) {
        Some(latest.to_string())
    } else {
        None
    }
}

/// Compare two semver strings. Returns true if `latest` is strictly newer than `current`.
fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> (u64, u64, u64) {
        let mut parts = s.split('.');
        let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
        let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
        let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    };
    parse(latest) > parse(current)
}

#[derive(Debug)]
pub struct HomeState {
    pub available_update: Option<String>,
    version_rx: Option<Receiver<Option<String>>>,
}

impl HomeState {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(check_brew_update());
        });
        Self {
            available_update: None,
            version_rx: Some(rx),
        }
    }

    pub fn tick_version_check(&mut self) {
        if let Some(ref rx) = self.version_rx {
            if let Ok(result) = rx.try_recv() {
                self.available_update = result;
                self.version_rx = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer() {
        assert!(is_newer("2.0.1", "2.0.0"));
        assert!(is_newer("2.1.0", "2.0.9"));
        assert!(is_newer("3.0.0", "2.9.9"));
        assert!(!is_newer("2.0.0", "2.0.0"));
        assert!(!is_newer("1.9.9", "2.0.0"));
        assert!(!is_newer("2.0.0", "2.0.1"));
    }
}
