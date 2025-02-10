use dirs;
use log::{error, info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use thiserror::Error;
use toml;

pub static HOME_DIR: Lazy<PathBuf> = Lazy::new(|| match dirs::home_dir() {
    Some(path) => path,
    None => {
        error!("HOME environment variable is not set. Exiting normally (exit code 0).");
        process::exit(0);
    }
});

pub static CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| {
    if let Ok(cfg) = env::var("NFD2NFC_CONFIG") {
        let trimmed = cfg.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    expand_tilde("~/.config/nfd2nfc/config.toml")
});

/// Raw configuration with unprocessed path strings from the config file.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct RawConfig {
    #[serde(default)]
    pub recursive_watch_paths: Vec<String>,
    #[serde(default)]
    pub non_recursive_watch_paths: Vec<String>,
    #[serde(default)]
    pub recursive_exclude_paths: Vec<String>,
}

/// Configuration refined by removing non-existent and inconsistent entries.
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub recursive_watch_paths: Vec<PathBuf>,
    pub non_recursive_watch_paths: Vec<PathBuf>,
    pub recursive_exclude_paths: Vec<PathBuf>,
}

impl From<RawConfig> for Config {
    fn from(raw: RawConfig) -> Self {
        // Convert raw string paths to validated absolute PathBufs using process_path.
        let rwp: Vec<PathBuf> = raw
            .recursive_watch_paths
            .iter()
            .filter_map(|s| process_path(s))
            .collect();
        let nrwp: Vec<PathBuf> = raw
            .non_recursive_watch_paths
            .iter()
            .filter_map(|s| process_path(s))
            .collect();
        let rex: Vec<PathBuf> = raw
            .recursive_exclude_paths
            .iter()
            .filter_map(|s| process_path(s))
            .collect();

        // Convert each vector into a HashSet.
        let rwp_set: HashSet<PathBuf> = rwp.into_iter().collect();
        let nrwp_set: HashSet<PathBuf> = nrwp.into_iter().collect();
        let rex_set: HashSet<PathBuf> = rex.into_iter().collect();

        // 1. Within-set deduplication:
        // For recursive watch paths, remove those that are subpaths of another recursive watch path.
        let rwp_set = remove_included_paths(&rwp_set);
        // For exclude paths, remove subpaths.
        let rex_set = remove_included_paths(&rex_set);
        // For non-recursive watch paths, we keep all (even if one is under another).

        // 2. Cross-set filtering:
        // For recursive watch paths: if a path is under any exclude path, remove it.
        let refined_rwp = filter_out_by_prefix(&rwp_set, &rex_set);

        // For non-recursive watch paths: remove any path that is under any recursive watch path.
        let nrwp_temp = filter_out_by_prefix(&nrwp_set, &rwp_set);
        // And remove those that are under any exclude path.
        let refined_nrwp = filter_out_by_prefix(&nrwp_temp, &rex_set);

        // Exclude paths remain as refined rex_set.
        let refined_rex = rex_set; // Already deduplicated.

        // Convert each set into a sorted Vec for deterministic ordering.
        let mut rwp_vec: Vec<PathBuf> = refined_rwp.into_iter().collect();
        let mut nrwp_vec: Vec<PathBuf> = refined_nrwp.into_iter().collect();
        let mut rex_vec: Vec<PathBuf> = refined_rex.into_iter().collect();

        rwp_vec.sort_by_key(|p| p.to_string_lossy().to_string());
        nrwp_vec.sort_by_key(|p| p.to_string_lossy().to_string());
        rex_vec.sort_by_key(|p| p.to_string_lossy().to_string());

        Config {
            recursive_watch_paths: rwp_vec,
            non_recursive_watch_paths: nrwp_vec,
            recursive_exclude_paths: rex_vec,
        }
    }
}

/// Removes paths that are subpaths of another path in the given set.
/// For any two distinct paths A and B, if A is a prefix of B, then B is removed.
fn remove_included_paths(set: &HashSet<PathBuf>) -> HashSet<PathBuf> {
    let mut result = set.clone();
    for a in set {
        for b in set {
            if a != b && b.starts_with(a) {
                result.remove(b);
            }
        }
    }
    result
}

/// Filters out from 'set' any path that is under any of the paths in 'prefixes'.
fn filter_out_by_prefix(set: &HashSet<PathBuf>, prefixes: &HashSet<PathBuf>) -> HashSet<PathBuf> {
    set.iter()
        .filter(|s| !prefixes.iter().any(|p| s.starts_with(p)))
        .cloned()
        .collect()
}

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
            recursive_exclude_paths: config
                .recursive_exclude_paths
                .into_iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
        }
    }
}

impl Config {
    pub fn save_to_file(&self, path: &Path) -> Result<(), ConfigError> {
        let raw: RawConfig = self.clone().into();
        let toml_content = toml::to_string_pretty(&raw).map_err(ConfigError::Serialize)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(ConfigError::Io)?;
        }
        fs::write(path, toml_content).map_err(ConfigError::Io)?;
        Ok(())
    }
}

/// Loads and refines the configuration.
/// If the configuration file is missing, returns the default configuration.
/// The refined settings are then saved back to the configuration file.
pub fn load_config() -> Result<Config, ConfigError> {
    let raw_config = match read_or_default_config(&CONFIG_PATH) {
        Ok(raw) => raw,
        Err(e) => {
            return Err(e);
        }
    };

    // Convert raw configuration to a refined configuration by removing
    // non-existent paths and any logically inconsistent settings.
    let config: Config = raw_config.into();
    info!("Configuration refined (removed invalid entries).");

    // Write the refined configuration back to the configuration file.
    match config.save_to_file(&CONFIG_PATH) {
        Ok(()) => info!("Refined configuration saved successfully."),
        Err(e) => {
            error!("Failed to apply refined settings to config file: {}", e);
            return Err(e);
        }
    }

    Ok(config)
}

fn read_or_default_config(path: &PathBuf) -> Result<RawConfig, ConfigError> {
    if path.exists() {
        info!("Configuration file found at {}.", path.display());
        read_config(path)
    } else {
        info!(
            "Configuration file not found at {}. Default configuration will be generated.",
            path.display()
        );
        Ok(RawConfig::default())
    }
}

fn read_config(path: &Path) -> Result<RawConfig, ConfigError> {
    let content = match fs::read_to_string(path) {
        Ok(c) => {
            info!("Configuration file read successfully.");
            c
        }
        Err(e) => {
            error!("Failed to read configuration file: {}", e);
            return Err(ConfigError::Io(e));
        }
    };

    let config = match toml::from_str(&content) {
        Ok(cfg) => {
            info!("Configuration parsed successfully.");
            cfg
        }
        Err(e) => {
            error!("Failed to parse configuration: {}", e);
            return Err(ConfigError::Parse(e));
        }
    };

    Ok(config)
}

fn process_path(path_str: &str) -> Option<PathBuf> {
    let trimmed = path_str.trim();
    if trimmed.is_empty() {
        warn!("Empty path detected; skipping.");
        return None;
    }
    let expanded_path = expand_tilde(trimmed);
    let canon_path = match fs::canonicalize(&expanded_path) {
        Ok(path) => path,
        Err(e) => {
            warn!(
                "Path not found or canonicalization failed: {}: {}",
                expanded_path.display(),
                e
            );
            return None;
        }
    };
    let metadata = match fs::metadata(&canon_path) {
        Ok(meta) => meta,
        Err(e) => {
            warn!(
                "Failed to read metadata for {}: {}",
                canon_path.display(),
                e
            );
            return None;
        }
    };
    if !metadata.is_dir() {
        warn!("{} is not a directory.", canon_path.display());
        return None;
    }
    Some(canon_path)
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        let mut home_path = HOME_DIR.clone();
        home_path.push(stripped);
        home_path
    } else {
        PathBuf::from(path)
    }
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
