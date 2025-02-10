use log::{error, info};
use notify::Event;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use tokio::fs;
use unicode_normalization::{is_nfc, UnicodeNormalization};

pub async fn handle_event(event: Event) {
    let path = match event.paths.get(0) {
        Some(p) => p,
        None => return,
    };

    let actual_name = match get_actual_file_name(path).await {
        Some(name) => name,
        None => {
            error!("Error retrieving file name: {:?}", path);
            return;
        }
    };

    if is_nfc(&actual_name) {
        return;
    }

    let nfc_file_name: String = actual_name.nfc().collect();
    let new_path = path.with_file_name(&nfc_file_name);

    match fs::rename(path, &new_path).await {
        Ok(()) => info!("Converted to NFC: {}", new_path.display()),
        Err(e) => error!("Conversion to NFC failed for {}: {}", new_path.display(), e),
    }
}

pub async fn get_actual_file_name(path: &Path) -> Option<String> {
    let parent = path.parent()?;
    let target_meta = fs::metadata(path).await.ok()?;
    let target_ino = target_meta.ino();
    let target_dev = target_meta.dev();
    let mut read_dir = fs::read_dir(parent).await.ok()?;
    while let Ok(Some(entry)) = read_dir.next_entry().await {
        if let Ok(meta) = entry.metadata().await {
            if meta.ino() == target_ino && meta.dev() == target_dev {
                return entry.file_name().into_string().ok();
            }
        }
    }
    None
}
