use log::{debug, error, info};
use rayon::prelude::*;
use std::collections::VecDeque;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use unicode_normalization::{is_nfc, is_nfd, UnicodeNormalization};

/// Heuristically convert a single file/folder name to NFC without scanning the parent directory.
/// This function does not verify the actual normalization by scanning the parent's contents;
/// if the name is likely in NFD, it renames it to NFC.
pub fn heuristic_normalize_name_to_nfc(target_path: &Path) {
    info!(
        "Starting heuristic conversion to NFC for: {}",
        target_path.display()
    );

    let target_name = match target_path.file_name() {
        Some(name) => name,
        None => {
            error!("Invalid file/folder name: {}", target_path.display());
            return;
        }
    };

    let target_name_str = target_name.to_string_lossy();
    let nfd_name: String = target_name_str.nfd().collect();
    let nfc_name: String = target_name_str.nfc().collect();

    if nfd_name == nfc_name {
        debug!("No conversion needed for: {}", target_path.display());
        return;
    }

    let nfc_path = target_path.with_file_name(nfc_name);
    let nfd_path = target_path.with_file_name(nfd_name);

    match fs::rename(&nfd_path, &nfc_path) {
        Ok(_) => info!("Heuristically converted {} to NFC", nfc_path.display()),
        Err(e) => error!(
            "Failed to heuristically convert {} to NFC: {}",
            target_path.display(),
            e
        ),
    }
}

/// Heuristically convert a single file/folder name to NFD without scanning the parent directory.
/// This function does not verify the actual normalization by scanning the parent's contents;
/// if the name is likely in NFC, it renames it to NFD.
pub fn heuristic_normalize_name_to_nfd(target_path: &Path) {
    info!(
        "Starting heuristic conversion to NFD for: {}",
        target_path.display()
    );

    let target_name = match target_path.file_name() {
        Some(name) => name,
        None => {
            error!("Invalid file/folder name: {}", target_path.display());
            return;
        }
    };

    let target_name_str = target_name.to_string_lossy();
    let nfd_name: String = target_name_str.nfd().collect();
    let nfc_name: String = target_name_str.nfc().collect();

    if nfd_name == nfc_name {
        debug!("No conversion needed for: {}", target_path.display());
        return;
    }

    let nfd_path = target_path.with_file_name(nfd_name);
    let nfc_path = target_path.with_file_name(nfc_name);

    match fs::rename(&nfc_path, &nfd_path) {
        Ok(_) => info!("Heuristically converted {} to NFD", nfd_path.display()),
        Err(e) => error!(
            "Failed to heuristically convert {} to NFD: {}",
            target_path.display(),
            e
        ),
    }
}

pub fn normalize_names_to_nfc(target_folder: &Path, recursive: bool) {
    info!(
        "Starting folder conversion to NFC for: {} (recursive: {})",
        target_folder.display(),
        recursive
    );
    let mut queue = VecDeque::new();
    queue.push_back(target_folder.to_path_buf());

    while let Some(current_dir) = queue.pop_front() {
        debug!("Processing directory: {}", current_dir.display());
        let entries: Vec<_> = match fs::read_dir(&current_dir) {
            Ok(entries) => entries.filter_map(|entry| entry.ok()).collect(),
            Err(e) => {
                error!("Failed to read directory {}: {}", current_dir.display(), e);
                continue;
            }
        };

        let subdirs: Vec<_> = entries
            .par_iter()
            .filter_map(|entry| {
                let path = entry.path();
                let mut new_path = path.clone();

                if let Some(name) = path.file_name() {
                    if name == "." || name == ".." {
                        debug!("Skipping dot entry: {}", path.display());
                        return None;
                    }

                    let original_name = name.to_string_lossy();
                    if !is_nfc(&original_name) {
                        let nfc_name: String = original_name.nfc().collect();
                        new_path = path.with_file_name(&nfc_name);
                        match fs::rename(&path, &new_path) {
                            Ok(_) => info!("Converted {} to NFC", new_path.display()),
                            Err(e) => {
                                error!("Failed to convert {} to NFC: {}", path.display(), e);
                                new_path = path.clone();
                            }
                        }
                    } else {
                        debug!("Entry already in NFC: {}", path.display());
                    }
                }

                if recursive && new_path.is_dir() {
                    if let Ok(metadata) = fs::symlink_metadata(&new_path) {
                        let is_symlink = metadata.file_type().is_symlink();
                        let is_different_fs = !is_same_filesystem(target_folder, &new_path);

                        if !is_symlink && !is_different_fs {
                            Some(new_path)
                        } else {
                            debug!(
                                "Skipping directory (symlink or different FS): {}",
                                new_path.display()
                            );
                            None
                        }
                    } else {
                        error!("Failed to get metadata for {}", new_path.display());
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if recursive {
            queue.extend(subdirs);
        }
    }
    info!(
        "Completed folder conversion to NFC for: {}",
        target_folder.display()
    );
}

pub fn normalize_names_to_nfd(target_folder: &Path, recursive: bool) {
    info!(
        "Starting folder conversion to NFD for: {} (recursive: {})",
        target_folder.display(),
        recursive
    );
    let mut queue = VecDeque::new();
    queue.push_back(target_folder.to_path_buf());

    while let Some(current_dir) = queue.pop_front() {
        debug!("Processing directory: {}", current_dir.display());
        let entries: Vec<_> = match fs::read_dir(&current_dir) {
            Ok(entries) => entries.filter_map(|entry| entry.ok()).collect(),
            Err(e) => {
                error!("Failed to read directory {}: {}", current_dir.display(), e);
                continue;
            }
        };

        let subdirs: Vec<_> = entries
            .par_iter()
            .filter_map(|entry| {
                let path = entry.path();
                let mut new_path = path.clone();

                if let Some(name) = path.file_name() {
                    if name == "." || name == ".." {
                        debug!("Skipping dot entry: {}", path.display());
                        return None;
                    }

                    let original_name = name.to_string_lossy();
                    if !is_nfd(&original_name) {
                        let nfd_name: String = original_name.nfd().collect();
                        new_path = path.with_file_name(&nfd_name);
                        match fs::rename(&path, &new_path) {
                            Ok(_) => info!("Converted {} to NFD", new_path.display()),
                            Err(e) => {
                                error!("Failed to convert {} to NFD: {}", path.display(), e);
                                new_path = path.clone();
                            }
                        }
                    } else {
                        debug!("Entry already in NFD: {}", path.display());
                    }
                }

                if recursive && new_path.is_dir() {
                    if let Ok(metadata) = fs::symlink_metadata(&new_path) {
                        let is_symlink = metadata.file_type().is_symlink();
                        let is_different_fs = !is_same_filesystem(target_folder, &new_path);

                        if !is_symlink && !is_different_fs {
                            Some(new_path)
                        } else {
                            debug!(
                                "Skipping directory (symlink or different FS): {}",
                                new_path.display()
                            );
                            None
                        }
                    } else {
                        error!("Failed to get metadata for {}", new_path.display());
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if recursive {
            queue.extend(subdirs);
        }
    }
    info!(
        "Completed folder conversion to NFD for: {}",
        target_folder.display()
    );
}

fn is_same_filesystem(original_path: &Path, new_path: &Path) -> bool {
    let original_dev = fs::metadata(original_path).map(|m| m.dev()).unwrap_or(0);
    let new_dev = fs::metadata(new_path).map(|m| m.dev()).unwrap_or(0);
    original_dev == new_dev
}
