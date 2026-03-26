use crate::constants::CONFIG_PATH;
use crate::utils::expand_tilde;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PathStatus {
    Active,
    NotFound,
    NotADirectory,
    PermissionDenied,
    Redundant(usize),
    Overridden(usize),
}

impl PathStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PathStatus::Active => "Active",
            PathStatus::NotFound => "Not Found",
            PathStatus::NotADirectory => "Not a Dir",
            PathStatus::PermissionDenied => "No Access",
            PathStatus::Redundant(_) => "Redundant",
            PathStatus::Overridden(_) => "Overridden",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            PathStatus::Active => "✓",
            PathStatus::Redundant(_) => "~",
            PathStatus::Overridden(_) => "!",
            PathStatus::NotFound | PathStatus::NotADirectory | PathStatus::PermissionDenied => "✗",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
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

    fn from_config_entry(cfe: ConfigFileEntry) -> Self {
        let (canonical, status) = validate_path(&cfe.path);
        Self {
            raw: cfe.path,
            action: cfe.action,
            mode: cfe.mode,
            canonical,
            status,
            overrides: None,
        }
    }

    fn to_config_entry(&self) -> ConfigFileEntry {
        ConfigFileEntry {
            path: self.raw.clone(),
            action: self.action,
            mode: self.mode,
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
pub(crate) fn validate_path(path_str: &str) -> (Option<PathBuf>, PathStatus) {
    let trimmed = path_str.trim();
    if trimmed.is_empty() {
        return (None, PathStatus::NotFound);
    }

    let expanded = expand_tilde(trimmed);
    let canonical = match fs::canonicalize(&expanded) {
        Ok(p) => p,
        Err(e) => return (None, io_error_to_status(&e)),
    };

    match fs::metadata(&canonical) {
        Ok(meta) => {
            if !meta.is_dir() {
                (None, PathStatus::NotADirectory)
            } else {
                (Some(canonical), PathStatus::Active)
            }
        }
        Err(e) => (None, io_error_to_status(&e)),
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

/// Find the most specific Active Recursive parent entry that covers the given canonical path.
fn find_fallback_parent(
    entries: &[PathEntry],
    statuses: &[PathStatus],
    idx: usize,
    canonical: &Path,
) -> Option<usize> {
    let mut best_index: Option<usize> = None;
    let mut best_len: usize = 0;
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
                if best_index.is_none() || jlen > best_len {
                    best_index = Some(j);
                    best_len = jlen;
                }
            }
        }
    }
    best_index
}

/// Compute redundancy statuses for all entries.
///
/// Phase 1 — Resolve exact-path duplicates (same canonical path):
///   For each group sharing a canonical path, determine a single winner:
///   - Winning action = action of the first entry (lowest index)
///   - Among entries with the winning action: prefer Recursive over Children, then lowest index
///   - Winner = Active; same-action others = Redundant(winner); different-action = Overridden(winner)
///
/// Phase 2 — Resolve parent-child relationships:
///   Processing order: by canonical path length (shortest first), same length by index.
///   For each Active entry E:
///   - Find fallback F: most specific Active Recursive entry that is a prefix of E
///   - F.action == E.action → Redundant(F)
///   - F.action != E.action → Active (override of F)
///   - No F → Active
pub(crate) fn compute_statuses(entries: &mut [PathEntry]) {
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

    // Phase 1: Resolve exact-path duplicates
    for i in 0..entries.len() {
        if !matches!(statuses[i], PathStatus::Active) {
            continue;
        }
        let Some(ref canonical_i) = entries[i].canonical else {
            continue;
        };

        // Collect group: all Active entries with the same canonical path
        let group: Vec<usize> = (i..entries.len())
            .filter(|&j| {
                matches!(statuses[j], PathStatus::Active)
                    && entries[j]
                        .canonical
                        .as_ref()
                        .is_some_and(|c| c == canonical_i)
            })
            .collect();

        if group.len() < 2 {
            continue;
        }

        // Winning action = first entry's action (lowest index in group)
        let winning_action = entries[group[0]].action;

        // Among entries with the winning action: prefer Recursive, then lowest index
        let winner = *group
            .iter()
            .filter(|&&j| entries[j].action == winning_action)
            .min_by_key(|&&j| {
                let mode_priority = match entries[j].mode {
                    PathMode::Recursive => 0u8,
                    PathMode::Children => 1u8,
                };
                (mode_priority, j)
            })
            .unwrap(); // at least group[0] has winning_action

        // Classify all non-winners
        for &j in &group {
            if j == winner {
                continue;
            }
            if entries[j].action != winning_action {
                statuses[j] = PathStatus::Overridden(winner);
            } else {
                statuses[j] = PathStatus::Redundant(winner);
            }
        }
    }

    // Phase 2: Resolve parent-child relationships
    let order = build_processing_order(entries);
    for &idx in &order {
        if !matches!(statuses[idx], PathStatus::Active) {
            continue;
        }
        let Some(canonical) = entries[idx].canonical.as_ref() else {
            continue;
        };

        if let Some(f_idx) = find_fallback_parent(entries, &statuses, idx, canonical) {
            if entries[f_idx].action == entries[idx].action {
                statuses[idx] = PathStatus::Redundant(f_idx);
            } else {
                statuses[idx] = PathStatus::Active;
                overrides[idx] = Some(f_idx);
            }
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
        return (Config::default(), None);
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return (Config::default(), Some(ConfigError::Io(e))),
    };

    let config_file: ConfigFile = match toml::from_str(&content) {
        Ok(c) => c,
        Err(e) => return (Config::default(), Some(ConfigError::Parse(e))),
    };

    let mut entries: Vec<PathEntry> = config_file
        .paths
        .into_iter()
        .map(PathEntry::from_config_entry)
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
            .filter_map(|e| {
                if !matches!(e.status, PathStatus::Active) {
                    return None;
                }
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
            paths: self.paths.iter().map(PathEntry::to_config_entry).collect(),
        };

        let toml_content = toml::to_string_pretty(&config_file)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, toml_content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a PathEntry with a synthetic canonical path (no filesystem access needed).
    fn make_entry(canonical: &str, action: PathAction, mode: PathMode) -> PathEntry {
        PathEntry {
            raw: canonical.to_string(),
            action,
            mode,
            canonical: Some(PathBuf::from(canonical)),
            status: PathStatus::Active,
            overrides: None,
        }
    }

    fn make_not_found(raw: &str) -> PathEntry {
        PathEntry {
            raw: raw.to_string(),
            action: PathAction::Watch,
            mode: PathMode::Recursive,
            canonical: None,
            status: PathStatus::NotFound,
            overrides: None,
        }
    }

    // --- 2 entries: same path ---

    #[test]
    fn duplicate_same_action_same_mode() {
        let mut entries = vec![
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
        ];
        compute_statuses(&mut entries);
        assert_eq!(entries[0].status, PathStatus::Active);
        assert_eq!(entries[1].status, PathStatus::Redundant(0));
    }

    #[test]
    fn duplicate_same_action_recursive_over_children() {
        // Children first, Recursive second — Recursive should win
        let mut entries = vec![
            make_entry("/a", PathAction::Watch, PathMode::Children),
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
        ];
        compute_statuses(&mut entries);
        assert_eq!(entries[0].status, PathStatus::Redundant(1));
        assert_eq!(entries[1].status, PathStatus::Active);
    }

    #[test]
    fn duplicate_same_action_children_under_recursive() {
        // Recursive first, Children second — Recursive keeps Active
        let mut entries = vec![
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
            make_entry("/a", PathAction::Watch, PathMode::Children),
        ];
        compute_statuses(&mut entries);
        assert_eq!(entries[0].status, PathStatus::Active);
        assert_eq!(entries[1].status, PathStatus::Redundant(0));
    }

    #[test]
    fn duplicate_different_action() {
        let mut entries = vec![
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
            make_entry("/a", PathAction::Ignore, PathMode::Recursive),
        ];
        compute_statuses(&mut entries);
        assert_eq!(entries[0].status, PathStatus::Active);
        assert_eq!(entries[1].status, PathStatus::Overridden(0));
    }

    #[test]
    fn duplicate_different_action_ignore_first() {
        let mut entries = vec![
            make_entry("/a", PathAction::Ignore, PathMode::Recursive),
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
        ];
        compute_statuses(&mut entries);
        assert_eq!(entries[0].status, PathStatus::Active);
        assert_eq!(entries[1].status, PathStatus::Overridden(0));
    }

    // --- 3+ entries: same path (group processing) ---

    #[test]
    fn triple_same_action_same_mode() {
        let mut entries = vec![
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
        ];
        compute_statuses(&mut entries);
        assert_eq!(entries[0].status, PathStatus::Active);
        assert_eq!(entries[1].status, PathStatus::Redundant(0));
        assert_eq!(entries[2].status, PathStatus::Redundant(0));
    }

    #[test]
    fn triple_children_children_recursive() {
        let mut entries = vec![
            make_entry("/a", PathAction::Watch, PathMode::Children),
            make_entry("/a", PathAction::Watch, PathMode::Children),
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
        ];
        compute_statuses(&mut entries);
        // Recursive (#2) wins
        assert_eq!(entries[0].status, PathStatus::Redundant(2));
        assert_eq!(entries[1].status, PathStatus::Redundant(2));
        assert_eq!(entries[2].status, PathStatus::Active);
    }

    #[test]
    fn triple_watch_ignore_watch() {
        let mut entries = vec![
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
            make_entry("/a", PathAction::Ignore, PathMode::Recursive),
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
        ];
        compute_statuses(&mut entries);
        assert_eq!(entries[0].status, PathStatus::Active);
        assert_eq!(entries[1].status, PathStatus::Overridden(0));
        assert_eq!(entries[2].status, PathStatus::Redundant(0));
    }

    #[test]
    fn triple_watch_children_ignore_watch_recursive() {
        // Watch/Children, Ignore/Recursive, Watch/Recursive
        // Winning action = Watch (first), winner = Watch/Recursive (#2)
        let mut entries = vec![
            make_entry("/a", PathAction::Watch, PathMode::Children),
            make_entry("/a", PathAction::Ignore, PathMode::Recursive),
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
        ];
        compute_statuses(&mut entries);
        assert_eq!(entries[0].status, PathStatus::Redundant(2));
        assert_eq!(entries[1].status, PathStatus::Overridden(2));
        assert_eq!(entries[2].status, PathStatus::Active);
    }

    // --- Parent-child relationships (Phase 2 regression) ---

    #[test]
    fn parent_child_same_action() {
        let mut entries = vec![
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
            make_entry("/a/b", PathAction::Watch, PathMode::Recursive),
        ];
        compute_statuses(&mut entries);
        assert_eq!(entries[0].status, PathStatus::Active);
        assert_eq!(entries[1].status, PathStatus::Redundant(0));
    }

    #[test]
    fn parent_child_different_action() {
        let mut entries = vec![
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
            make_entry("/a/b", PathAction::Ignore, PathMode::Recursive),
        ];
        compute_statuses(&mut entries);
        assert_eq!(entries[0].status, PathStatus::Active);
        assert_eq!(entries[1].status, PathStatus::Active);
        assert_eq!(entries[1].overrides, Some(0));
    }

    #[test]
    fn children_mode_parent_no_coverage() {
        // Children mode parent does not cover child subdirectories
        let mut entries = vec![
            make_entry("/a", PathAction::Watch, PathMode::Children),
            make_entry("/a/b", PathAction::Watch, PathMode::Recursive),
        ];
        compute_statuses(&mut entries);
        assert_eq!(entries[0].status, PathStatus::Active);
        assert_eq!(entries[1].status, PathStatus::Active);
        assert!(entries[1].overrides.is_none());
    }

    // --- Edge cases ---

    #[test]
    fn empty_entries() {
        let mut entries: Vec<PathEntry> = vec![];
        compute_statuses(&mut entries);
        // No panic
    }

    #[test]
    fn single_entry() {
        let mut entries = vec![make_entry("/a", PathAction::Watch, PathMode::Recursive)];
        compute_statuses(&mut entries);
        assert_eq!(entries[0].status, PathStatus::Active);
        assert!(entries[0].overrides.is_none());
    }

    #[test]
    fn not_found_entry_keeps_status() {
        let mut entries = vec![
            make_entry("/a", PathAction::Watch, PathMode::Recursive),
            make_not_found("/nonexistent"),
        ];
        compute_statuses(&mut entries);
        assert_eq!(entries[0].status, PathStatus::Active);
        assert_eq!(entries[1].status, PathStatus::NotFound);
    }
}
