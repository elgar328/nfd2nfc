use log::{error, info};
use notify::Event;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use unicode_normalization::{is_nfc, UnicodeNormalization};

pub async fn handle_event(event: Event) {
    let path = match event.paths.get(0) {
        Some(p) => p,
        None => return,
    };

    let actual_name = match get_actual_file_name(path).await {
        Some(name) => name,
        None => {
            error!("Failed to retrieve file name for path: {}", path.display());
            return;
        }
    };

    if is_nfc(&actual_name) {
        return;
    }

    let nfc_file_name: String = actual_name.nfc().collect();
    let new_path = path.with_file_name(&nfc_file_name);

    match tokio::fs::rename(path, &new_path).await {
        Ok(()) => info!(
            "Converted to NFC: {}",
            new_path.to_string_lossy().nfc().collect::<String>()
        ),
        Err(e) => error!("Failed to convert {} to NFC: {}", new_path.display(), e),
    }
}

pub async fn get_actual_file_name(path: &Path) -> Option<String> {
    let parent = match path.parent() {
        Some(p) => p,
        None => {
            error!(
                "Failed to get parent directory for path: {}",
                path.display()
            );
            return None;
        }
    };
    let target_meta = match tokio::fs::symlink_metadata(path).await {
        Ok(meta) => meta,
        Err(e) => {
            error!("Failed to get metadata for path {}: {}", path.display(), e);
            return None;
        }
    };
    let target_ino = target_meta.ino();
    let target_dev = target_meta.dev();
    let mut read_dir = match tokio::fs::read_dir(parent).await {
        Ok(rd) => rd,
        Err(e) => {
            error!("Failed to read directory {}: {}", parent.display(), e);
            return None;
        }
    };
    while let Ok(Some(entry)) = read_dir.next_entry().await {
        match tokio::fs::symlink_metadata(entry.path()).await {
            Ok(meta) => {
                if meta.ino() == target_ino && meta.dev() == target_dev {
                    return match entry.file_name().into_string() {
                        Ok(name) => Some(name),
                        Err(os_str) => {
                            error!(
                                "Failed to convert file name to string: {}",
                                os_str.to_string_lossy()
                            );
                            None
                        }
                    };
                }
            }
            Err(e) => {
                error!(
                    "Failed to get metadata for directory entry {}: {}",
                    entry.path().display(),
                    e
                );
            }
        }
    }
    None
}
