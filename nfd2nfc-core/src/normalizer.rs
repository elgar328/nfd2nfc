use crate::utils::abbreviate_home_path;
use log::{debug, error, info};
use rayon::prelude::*;
use std::collections::VecDeque;
use std::ffi::CStr;
use std::fs::{self, File};
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use thiserror::Error;
use unicode_normalization::{is_nfc, is_nfd, UnicodeNormalization};

/// Target normalization form for filename conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum NormalizationTarget {
    NFC,
    NFD,
}

impl NormalizationTarget {
    /// Returns the string representation of the normalization target.
    pub fn as_str(&self) -> &'static str {
        match self {
            NormalizationTarget::NFC => "NFC",
            NormalizationTarget::NFD => "NFD",
        }
    }

    /// Convert a filename to this normalization form.
    pub fn convert(&self, name: &str) -> String {
        match self {
            NormalizationTarget::NFC => name.nfc().collect(),
            NormalizationTarget::NFD => name.nfd().collect(),
        }
    }

    /// Check if a name needs conversion to this normalization form.
    pub fn needs_conversion(&self, name: &str) -> bool {
        match self {
            NormalizationTarget::NFC => !is_nfc(name),
            NormalizationTarget::NFD => !is_nfd(name),
        }
    }
}

/// Errors that can occur during normalization operations.
#[derive(Debug, Error)]
pub enum NormalizerError {
    #[error("Invalid file/folder name: {0}")]
    InvalidName(String),

    #[error("Failed to rename '{from}' to '{to}': {source}")]
    RenameError {
        from: String,
        to: String,
        source: std::io::Error,
    },

    #[error("Failed to read directory '{0}': {1}")]
    ReadDirError(String, std::io::Error),

    #[error("Failed to open file '{0}': {1}")]
    OpenError(String, std::io::Error),

    #[error("Failed to get actual path: fcntl F_GETPATH failed")]
    FcntlError,

    #[error("Failed to convert path to UTF-8")]
    Utf8Error,
}

impl NormalizerError {
    pub fn is_not_found(&self) -> bool {
        matches!(self, NormalizerError::OpenError(_, e) if e.kind() == std::io::ErrorKind::NotFound)
    }
}

/// Get the actual file name as stored on disk using macOS fcntl F_GETPATH.
/// This is O(1) compared to O(n) directory scanning.
///
/// This is necessary because filesystem events may report paths in a different
/// Unicode normalization form than what's actually stored on disk.
pub fn get_actual_file_name(path: &Path) -> Result<String, NormalizerError> {
    let file =
        File::open(path).map_err(|e| NormalizerError::OpenError(path.display().to_string(), e))?;

    let mut buf = [0u8; libc::PATH_MAX as usize];
    let ret = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETPATH, buf.as_mut_ptr()) };

    if ret == -1 {
        return Err(NormalizerError::FcntlError);
    }

    let c_str = unsafe { CStr::from_ptr(buf.as_ptr() as *const i8) };
    let full_path_str = c_str.to_str().map_err(|_| NormalizerError::Utf8Error)?;

    Path::new(full_path_str)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| NormalizerError::InvalidName(path.display().to_string()))
}

/// Normalize a single file/folder name to the target normalization form.
///
/// This function uses `get_actual_file_name` to get the real filename from disk,
/// then renames it if conversion is needed.
pub fn normalize_single_file(
    target_path: &Path,
    target: NormalizationTarget,
) -> Result<(), NormalizerError> {
    info!(
        "Starting single file conversion to {} for: {}",
        target.as_str(),
        abbreviate_home_path(target_path)
    );

    let actual_name = get_actual_file_name(target_path)?;

    if !target.needs_conversion(&actual_name) {
        debug!("No conversion needed for: {}", target_path.display());
        return Ok(());
    }

    let new_name = target.convert(&actual_name);
    let new_path = target_path.with_file_name(&new_name);

    fs::rename(target_path, &new_path).map_err(|e| NormalizerError::RenameError {
        from: target_path.display().to_string(),
        to: new_path.display().to_string(),
        source: e,
    })?;

    info!(
        "Converted {} to {}",
        abbreviate_home_path(&new_path),
        target.as_str()
    );

    Ok(())
}

/// Normalize filenames in a directory to the target normalization form.
///
/// If `recursive` is true, subdirectories are also processed.
/// Symlinks and directories on different filesystems are skipped.
pub fn normalize_directory(
    target_folder: &Path,
    recursive: bool,
    target: NormalizationTarget,
) -> Result<(), NormalizerError> {
    info!(
        "Starting folder conversion to {} for: {} (recursive: {})",
        target.as_str(),
        abbreviate_home_path(target_folder),
        recursive
    );

    let mut queue = VecDeque::new();
    queue.push_back(target_folder.to_path_buf());

    while let Some(current_dir) = queue.pop_front() {
        debug!(
            "Processing directory: {}",
            abbreviate_home_path(&current_dir)
        );

        let entries: Vec<_> = match fs::read_dir(&current_dir) {
            Ok(entries) => entries.filter_map(|entry| entry.ok()).collect(),
            Err(e) => {
                error!(
                    "Failed to read directory {}: {}",
                    abbreviate_home_path(&current_dir),
                    e
                );
                continue;
            }
        };

        let subdirs: Vec<_> = entries
            .par_iter()
            .filter_map(|entry| {
                let path = entry.path();

                let name = match path.file_name() {
                    Some(n) => n,
                    None => return None,
                };

                if name == "." || name == ".." {
                    debug!("Skipping dot entry: {}", path.display());
                    return None;
                }

                let original_name = name.to_string_lossy();

                let new_path = if target.needs_conversion(&original_name) {
                    let new_name = target.convert(&original_name);
                    let renamed_path = path.with_file_name(&new_name);
                    match fs::rename(&path, &renamed_path) {
                        Ok(_) => {
                            info!(
                                "Converted {} to {}",
                                abbreviate_home_path(&renamed_path),
                                target.as_str()
                            );
                            renamed_path
                        }
                        Err(e) => {
                            error!(
                                "Failed to convert {} to {}: {}",
                                abbreviate_home_path(&path),
                                target.as_str(),
                                e
                            );
                            path
                        }
                    }
                } else {
                    debug!(
                        "Entry already in {}: {}",
                        target.as_str(),
                        abbreviate_home_path(&path)
                    );
                    path
                };

                // Check if we should recurse into this directory
                if !(recursive && new_path.is_dir()) {
                    return None;
                }
                let metadata = match fs::symlink_metadata(&new_path) {
                    Ok(m) => m,
                    Err(_) => {
                        error!(
                            "Failed to get metadata for {}",
                            abbreviate_home_path(&new_path)
                        );
                        return None;
                    }
                };
                if metadata.file_type().is_symlink()
                    || !is_same_filesystem(target_folder, &new_path)
                {
                    debug!(
                        "Skipping directory (symlink or different FS): {}",
                        new_path.display()
                    );
                    return None;
                }
                Some(new_path)
            })
            .collect();

        if recursive {
            queue.extend(subdirs);
        }
    }

    info!(
        "Completed folder conversion to {} for: {}",
        target.as_str(),
        abbreviate_home_path(target_folder)
    );

    Ok(())
}

/// Check if two paths are on the same filesystem.
#[cfg(unix)]
fn is_same_filesystem(original_path: &Path, new_path: &Path) -> bool {
    let original_dev = fs::metadata(original_path).map(|m| m.dev()).unwrap_or(0);
    let new_dev = fs::metadata(new_path).map(|m| m.dev()).unwrap_or(0);
    original_dev == new_dev
}

#[cfg(not(unix))]
fn is_same_filesystem(_original_path: &Path, _new_path: &Path) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::path::PathBuf;
    use tempfile::TempDir;
    use unicode_normalization::UnicodeNormalization;

    /// Create a file with NFD filename in the given directory.
    fn create_nfd_file(dir: &Path, name: &str) -> PathBuf {
        let nfd_name: String = name.nfd().collect();
        let path = dir.join(&nfd_name);
        File::create(&path).unwrap();
        path
    }

    #[test]
    fn test_get_actual_file_name_returns_nfd() {
        let temp = TempDir::new().unwrap();
        let nfd_name: String = "카페.txt".nfd().collect();
        let path = temp.path().join(&nfd_name);
        File::create(&path).unwrap();

        let actual = get_actual_file_name(&path).unwrap();
        assert!(!is_nfc(&actual), "Should return NFD name from disk");
    }

    #[test]
    fn test_normalize_single_file_converts_to_nfc() {
        let temp = TempDir::new().unwrap();
        let path = create_nfd_file(temp.path(), "테스트.txt");

        normalize_single_file(&path, NormalizationTarget::NFC).unwrap();

        let entries: Vec<_> = fs::read_dir(temp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 1);

        let name = entries[0].file_name().to_string_lossy().to_string();
        assert!(is_nfc(&name), "File should be NFC after conversion");
    }

    #[test]
    fn test_normalize_directory_recursive() {
        let temp = TempDir::new().unwrap();

        // Create nested structure with NFD names
        let sub = temp.path().join("서브폴더".nfd().collect::<String>());
        fs::create_dir(&sub).unwrap();
        create_nfd_file(temp.path(), "파일1.txt");
        create_nfd_file(&sub, "파일2.txt");

        normalize_directory(temp.path(), true, NormalizationTarget::NFC).unwrap();

        // Verify all entries are NFC
        fn check_all_nfc(dir: &Path) -> bool {
            for entry in fs::read_dir(dir).unwrap().filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if !is_nfc(&name) {
                    return false;
                }
                if entry.path().is_dir() && !check_all_nfc(&entry.path()) {
                    return false;
                }
            }
            true
        }

        assert!(check_all_nfc(temp.path()));
    }

    #[test]
    fn test_no_conversion_needed() {
        let temp = TempDir::new().unwrap();
        let nfc_name = "already_nfc.txt";
        let path = temp.path().join(nfc_name);
        File::create(&path).unwrap();

        // Should succeed without error
        normalize_single_file(&path, NormalizationTarget::NFC).unwrap();

        // File should still exist with same name
        assert!(path.exists());
    }
}
