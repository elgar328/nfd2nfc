use crate::constants::CONFIG_PATH;
use crate::utils::expand_tilde;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use toml;

/// Raw configuration with unprocessed path strings from the config file.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct RawConfig {
    #[serde(default)]
    pub recursive_watch_paths: Vec<String>,
    #[serde(default)]
    pub non_recursive_watch_paths: Vec<String>,
    #[serde(default)]
    pub recursive_ignore_paths: Vec<String>,
}

/// Refined configuration with validated and canonical PathBuf entries.
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub recursive_watch_paths: Vec<PathBuf>,
    pub non_recursive_watch_paths: Vec<PathBuf>,
    pub recursive_ignore_paths: Vec<PathBuf>,
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

impl Config {
    /// Saves the refined configuration to the given path.
    pub fn save_to_file(&self, path: &Path) -> Result<(), ConfigError> {
        let raw: RawConfig = self.clone().into();
        raw.save_to_file(path)
    }
}

impl RawConfig {
    /// Saves the raw configuration to the given path.
    pub fn save_to_file(&self, path: &Path) -> Result<(), ConfigError> {
        let toml_content = toml::to_string_pretty(self).map_err(ConfigError::Serialize)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(ConfigError::Io)?;
        }
        fs::write(path, toml_content).map_err(ConfigError::Io)?;
        Ok(())
    }
}

/// Removes subpaths from the given list of paths.
/// The input paths are first sorted (lexicographically), then
/// any path that is a subpath of its predecessor is removed.
fn remove_subpaths(mut paths: Vec<PathBuf>, section: &str) -> Vec<PathBuf> {
    paths.sort_by_key(|p| p.to_string_lossy().to_string());
    let mut output: Vec<PathBuf> = Vec::new();
    for path in paths {
        if let Some(prev) = output.last() {
            if path.starts_with(prev) {
                warn!(
                    " - Removed {} path '{}' because it is a subpath of '{}'.",
                    section,
                    path.to_string_lossy(),
                    prev.to_string_lossy()
                );
                continue;
            }
        }
        output.push(path);
    }
    output
}

fn remove_duplicates(mut paths: Vec<PathBuf>, section: &str) -> Vec<PathBuf> {
    // Sort paths lexicographically
    paths.sort_by_key(|p| p.to_string_lossy().to_string());
    let mut output = Vec::new();
    for path in paths {
        if let Some(last) = output.last() {
            if last == &path {
                info!(
                    " - Removed duplicate {} path: {}",
                    section,
                    path.to_string_lossy()
                );
                continue;
            }
        }
        output.push(path);
    }
    output
}

/// Filters out any paths from `paths` that are subpaths of any path in `prefixes`.
fn filter_by_prefixes(
    paths: Vec<PathBuf>,
    prefixes: &Vec<PathBuf>,
    section: &str,
    conflict_with: &str,
) -> Vec<PathBuf> {
    paths
        .into_iter()
        .filter(|p| {
            for prefix in prefixes {
                if p.starts_with(prefix) {
                    warn!(
                        " - Removed {} path '{}' as it is a subpath of {} path '{}'.",
                        section,
                        p.to_string_lossy(),
                        conflict_with,
                        prefix.to_string_lossy()
                    );
                    return false;
                }
            }
            true
        })
        .collect()
}

/// Converts RawConfig into a refined Config by validating and filtering paths,
/// and logs any removals due to invalidity, duplication, or conflicts.
impl From<RawConfig> for Config {
    fn from(raw: RawConfig) -> Self {
        // Step 1: Refine each section individually.
        let rwp = canonicalize_paths(&raw.recursive_watch_paths, "recursive watch");
        let nrwp = canonicalize_paths(&raw.non_recursive_watch_paths, "non-recursive watch");
        let rip = canonicalize_paths(&raw.recursive_ignore_paths, "ignore");

        // Step 2: Remove subpaths within each section.
        let rwp = remove_duplicates(rwp, "recursive watch");
        let rwp = remove_subpaths(rwp, "recursive watch");
        let nrwp = remove_duplicates(nrwp, "non-recursive watch");
        let rip = remove_duplicates(rip, "ignore");
        let rip = remove_subpaths(rip, "ignore");

        // Step 3: Cross-set filtering.
        // For recursive watch paths, remove any that conflict with ignore paths.
        let rwp = filter_by_prefixes(rwp, &rip, "recursive watch", "ignore");
        // For non-recursive watch paths, remove those that conflict with recursive watch or ignore paths.
        let nrwp = filter_by_prefixes(nrwp, &rwp, "non-recursive watch", "recursive watch");
        let nrwp = filter_by_prefixes(nrwp, &rip, "non-recursive watch", "ignore");

        Config {
            recursive_watch_paths: rwp,
            non_recursive_watch_paths: nrwp,
            recursive_ignore_paths: rip,
        }
    }
}

/// Converts a Config back into RawConfig by converting PathBuf's to strings.
impl From<Config> for RawConfig {
    fn from(config: Config) -> Self {
        RawConfig {
            recursive_watch_paths: config
                .recursive_watch_paths
                .into_iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
            non_recursive_watch_paths: config
                .non_recursive_watch_paths
                .into_iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
            recursive_ignore_paths: config
                .recursive_ignore_paths
                .into_iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
        }
    }
}

/// Reads the config file from CONFIG_PATH if it exists, or returns the default RawConfig.
pub fn read_or_default_config(path: &PathBuf) -> Result<RawConfig, ConfigError> {
    if path.exists() {
        info!(" + Found config file at {}.", path.display());
        read_config(path)
    } else {
        info!(
            " + No config file at {}. Default configuration will be used.",
            path.display()
        );
        Ok(RawConfig::default())
    }
}

/// Reads and parses the config file from the given path.
fn read_config(path: &Path) -> Result<RawConfig, ConfigError> {
    let content = fs::read_to_string(path).map_err(ConfigError::Io)?;
    info!(" + Config file loaded successfully.");
    let config = toml::from_str(&content).map_err(ConfigError::Parse)?;
    info!(" + Config parsed successfully.");
    Ok(config)
}

/// Loads and refines the configuration.
/// This function reads config.toml, processes the paths (validates, removes duplicates and conflicts),
/// saves the refined configuration back to the file, and returns the refined Config.
/// Intermediate steps and removals are logged.
pub fn load_config() -> Result<Config, ConfigError> {
    let raw_config = read_or_default_config(&CONFIG_PATH)?;
    let config: Config = raw_config.into();
    info!(" + Config refined (invalid entries removed).");
    config.save_to_file(&CONFIG_PATH)?;
    info!(" + Refined config saved.");
    Ok(config)
}

/// Refines a list of raw path strings for a given section (e.g., "recursive watch").
/// Invalid or empty paths are skipped with a warning.
fn canonicalize_paths(raw_paths: &Vec<String>, section: &str) -> Vec<PathBuf> {
    let mut valid_paths = Vec::new();
    for s in raw_paths {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            warn!(" - Removed empty {} path.", section);
            continue;
        }
        match process_path(trimmed) {
            Some(p) => valid_paths.push(p),
            None => warn!(" - Removed invalid {} path: {}", section, s),
        }
    }
    valid_paths
}

/// Converts a path string to its canonical PathBuf.
/// Returns None if conversion fails.
fn process_path(path_str: &str) -> Option<PathBuf> {
    let expanded_path = expand_tilde(path_str);
    let canon_path = match fs::canonicalize(&expanded_path) {
        Ok(p) => p,
        Err(e) => {
            debug!(
                "Canonicalization failed for {}: {}",
                expanded_path.display(),
                e
            );
            return None;
        }
    };
    match fs::metadata(&canon_path) {
        Ok(meta) => {
            if !meta.is_dir() {
                debug!("{} is not a directory; skipping.", canon_path.display());
                return None;
            }
        }
        Err(e) => {
            debug!(
                "Failed to read metadata for {}: {}",
                canon_path.display(),
                e
            );
            return None;
        }
    }
    Some(canon_path)
}
