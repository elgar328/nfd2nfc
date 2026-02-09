use crate::constants::CONFIG_PATH;
use crate::utils::expand_tilde;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

// === TOML serde types (private) ===

#[derive(Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    #[serde(default)]
    paths: Vec<ConfigFileEntry>,
}

#[derive(Deserialize, Serialize)]
struct ConfigFileEntry {
    path: String,
    action: PathAction,
    mode: PathMode,
}

// === Public types ===

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PathAction {
    Watch,
    Ignore,
}

impl PathAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            PathAction::Watch => "Watch",
            PathAction::Ignore => "Ignore",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            PathAction::Watch => PathAction::Ignore,
            PathAction::Ignore => PathAction::Watch,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PathMode {
    Recursive,
    Children,
}

impl PathMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            PathMode::Recursive => "Recursive",
            PathMode::Children => "Children",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            PathMode::Recursive => PathMode::Children,
            PathMode::Children => PathMode::Recursive,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStatus {
    Active,
    NotFound,
    NotADirectory,
    PermissionDenied,
    Redundant(usize),
}

impl PathStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PathStatus::Active => "Active",
            PathStatus::NotFound => "Not Found",
            PathStatus::NotADirectory => "Not a Dir",
            PathStatus::PermissionDenied => "No Access",
            PathStatus::Redundant(_) => "Redundant",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            PathStatus::Active => "✓",
            PathStatus::Redundant(_) => "~",
            PathStatus::NotFound | PathStatus::NotADirectory | PathStatus::PermissionDenied => "✗",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PathEntry {
    pub raw: String,
    pub action: PathAction,
    pub mode: PathMode,
    pub canonical: Option<PathBuf>,
    pub status: PathStatus,
    pub overrides: Option<usize>,
}

impl PathEntry {
    /// Create a new PathEntry from a PathBuf, running validate_path automatically.
    pub fn new(path: PathBuf, action: PathAction, mode: PathMode) -> Self {
        let raw = path.to_string_lossy().to_string();
        let (canonical, status) = validate_path(&raw);
        Self {
            raw,
            action,
            mode,
            canonical,
            status,
            overrides: None,
        }
    }
}

/// Active entry summary for watcher event filtering.
#[derive(Debug, Clone)]
pub struct ActiveEntry {
    pub canonical: PathBuf,
    pub action: PathAction,
    pub mode: PathMode,
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub paths: Vec<PathEntry>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("Serialization error: {0}")]
    Serialize(#[from] toml::ser::Error),
}

impl ConfigError {
    pub fn user_message(&self) -> &'static str {
        match self {
            ConfigError::Io(_) => "Failed to read config file",
            ConfigError::Parse(_) => "Config file format is invalid",
            ConfigError::Serialize(_) => "Failed to save config file",
        }
    }
}

// === Functions ===

/// Map an IO error to the corresponding PathStatus.
fn io_error_to_status(e: &std::io::Error) -> PathStatus {
    match e.kind() {
        std::io::ErrorKind::PermissionDenied => PathStatus::PermissionDenied,
        _ => PathStatus::NotFound,
    }
}

/// Validate a single path string. Returns (canonical PathBuf if valid, status).
pub fn validate_path(path_str: &str) -> (Option<PathBuf>, PathStatus) {
    let trimmed = path_str.trim();
    if trimmed.is_empty() {
        return (None, PathStatus::NotFound);
    }

    let expanded = expand_tilde(trimmed);
    let canonical = match fs::canonicalize(&expanded) {
        Ok(p) => p,
        Err(e) => {
            debug!("Canonicalization failed for {}: {}", expanded.display(), e);
            return (None, io_error_to_status(&e));
        }
    };

    match fs::metadata(&canonical) {
        Ok(meta) => {
            if !meta.is_dir() {
                (None, PathStatus::NotADirectory)
            } else {
                (Some(canonical), PathStatus::Active)
            }
        }
        Err(e) => {
            debug!("Metadata read failed for {}: {}", canonical.display(), e);
            (None, io_error_to_status(&e))
        }
    }
}

/// Build processing order: indices sorted by canonical path length (shortest first),
/// then by original index for stable ordering.
fn build_processing_order(entries: &[PathEntry]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..entries.len()).collect();
    let canonical_len = |idx: usize| {
        entries[idx]
            .canonical
            .as_ref()
            .map(|p| p.as_os_str().len())
            .unwrap_or(0)
    };
    order.sort_by(|&a, &b| canonical_len(a).cmp(&canonical_len(b)).then(a.cmp(&b)));
    order
}

/// Check if another entry at a lower index has the same canonical path and is Active.
fn find_duplicate(
    entries: &[PathEntry],
    statuses: &[PathStatus],
    idx: usize,
    canonical: &Path,
) -> Option<usize> {
    (0..idx).find(|&j| {
        entries[j]
            .canonical
            .as_ref()
            .is_some_and(|jc| *jc == *canonical && matches!(statuses[j], PathStatus::Active))
    })
}

/// Find the most specific Active Recursive parent entry that covers the given canonical path.
fn find_fallback_parent(
    entries: &[PathEntry],
    statuses: &[PathStatus],
    idx: usize,
    canonical: &Path,
) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None; // (index, path_len)
    for (j, entry_j) in entries.iter().enumerate() {
        if j == idx || !matches!(statuses[j], PathStatus::Active) {
            continue;
        }
        if let Some(ref jc) = entry_j.canonical {
            let is_parent = matches!(entry_j.mode, PathMode::Recursive)
                && *jc != *canonical
                && canonical.starts_with(jc);
            if is_parent {
                let jlen = jc.as_os_str().len();
                if best.is_none_or(|(_, best_len)| jlen > best_len) {
                    best = Some((j, jlen));
                }
            }
        }
    }
    best.map(|(idx, _)| idx)
}

/// Compute redundancy statuses for all entries.
///
/// Processing order: by canonical path length (shortest first), same length by index.
/// For each valid entry E:
/// 1. Same canonical path at a lower index and Active → Redundant(that index)
/// 2. Find fallback F: most specific Active Recursive entry (excluding E) that is a prefix of E
/// 3. F exists and F.action == E.action → Redundant(F's index)
/// 4. F exists and F.action != E.action → Active (exception)
/// 5. No F → Active
pub fn compute_statuses(entries: &mut [PathEntry]) {
    let order = build_processing_order(entries);

    // Track statuses and overrides separately to avoid borrow issues.
    let mut statuses: Vec<PathStatus> = entries
        .iter()
        .map(|e| {
            if e.canonical.is_some() {
                PathStatus::Active // tentative
            } else {
                e.status
            }
        })
        .collect();
    let mut overrides: Vec<Option<usize>> = vec![None; entries.len()];

    for &idx in &order {
        let Some(canonical) = entries[idx].canonical.as_ref() else {
            continue;
        };

        // Rule 1: same canonical path at lower index that is Active
        if let Some(dup_idx) = find_duplicate(entries, &statuses, idx, canonical) {
            statuses[idx] = PathStatus::Redundant(dup_idx);
            continue;
        }

        // Rule 2-5: find fallback parent and determine status
        if let Some(f_idx) = find_fallback_parent(entries, &statuses, idx, canonical) {
            if entries[f_idx].action == entries[idx].action {
                statuses[idx] = PathStatus::Redundant(f_idx);
            } else {
                statuses[idx] = PathStatus::Active;
                overrides[idx] = Some(f_idx);
            }
        } else {
            statuses[idx] = PathStatus::Active;
        }
    }

    // Apply computed statuses and overrides
    for i in 0..entries.len() {
        entries[i].status = statuses[i];
        entries[i].overrides = overrides[i];
    }
}

/// Load configuration from the default config path.
/// Returns (Config, Option<ConfigError>).
/// File not found → empty config, no error (normal for new installs).
/// Parse error → empty config with error.
pub fn load_config() -> (Config, Option<ConfigError>) {
    let path = &*CONFIG_PATH;

    if !path.exists() {
        debug!(
            "No config file at {}. Default configuration will be used.",
            path.display()
        );
        return (Config::default(), None);
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read config file: {}", e);
            return (Config::default(), Some(ConfigError::Io(e)));
        }
    };

    let config_file: ConfigFile = match toml::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to parse config file: {}", e);
            return (Config::default(), Some(ConfigError::Parse(e)));
        }
    };

    let mut entries: Vec<PathEntry> = config_file
        .paths
        .into_iter()
        .map(|cfe| {
            let (canonical, status) = validate_path(&cfe.path);
            PathEntry {
                raw: cfe.path,
                action: cfe.action,
                mode: cfe.mode,
                canonical,
                status,
                overrides: None,
            }
        })
        .collect();

    compute_statuses(&mut entries);

    (Config { paths: entries }, None)
}

impl Config {
    /// Re-validate all paths and recompute redundancy statuses.
    pub fn refresh_statuses(&mut self) {
        for entry in &mut self.paths {
            let (canonical, status) = validate_path(&entry.raw);
            entry.canonical = canonical;
            entry.status = status;
            entry.overrides = None;
        }
        compute_statuses(&mut self.paths);
    }

    /// Extract active entries for watcher event filtering.
    pub fn active_entries(&self) -> Vec<ActiveEntry> {
        self.paths
            .iter()
            .filter(|e| matches!(e.status, PathStatus::Active))
            .filter_map(|e| {
                e.canonical.as_ref().map(|c| ActiveEntry {
                    canonical: c.clone(),
                    action: e.action,
                    mode: e.mode,
                })
            })
            .collect()
    }

    /// Save config to file in [[paths]] TOML format.
    pub fn save_to_file(&self, path: &Path) -> Result<(), ConfigError> {
        let config_file = ConfigFile {
            paths: self
                .paths
                .iter()
                .map(|e| ConfigFileEntry {
                    path: e.raw.clone(),
                    action: e.action,
                    mode: e.mode,
                })
                .collect(),
        };

        let toml_content = toml::to_string_pretty(&config_file)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, toml_content)?;
        Ok(())
    }
}
