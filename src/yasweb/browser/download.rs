use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tracing::info;

lazy_static::lazy_static! {
    static ref GLOBAL_DOWNLOAD_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);
}

pub fn get_global_download_dir() -> Arc<Mutex<Option<PathBuf>>> {
    Arc::new(Mutex::new(None)) // To decouple, pass this from caller
}

pub type DownloadSnapshot = HashMap<PathBuf, (u64, Option<SystemTime>)>;

pub fn configure_download_directory(
    tab: &Arc<headless_chrome::Tab>,
    download_dir: Option<&PathBuf>,
) -> Result<()> {
    if let Some(dl_dir) = download_dir {
        info!("Configuring download directory to {:?}", dl_dir);
        tab.call_method(headless_chrome::protocol::cdp::Page::SetDownloadBehavior {
            behavior: headless_chrome::protocol::cdp::Page::SetDownloadBehaviorBehaviorOption::Allow,
            download_path: Some(dl_dir.to_string_lossy().to_string()),
        })
        .context("Failed to configure browser download directory")?;
    }
    Ok(())
}

/// Captures spreadsheet files already present before the current report export.
pub fn snapshot_downloads(download_dir: Option<&PathBuf>) -> Result<DownloadSnapshot> {
    let mut snapshot = DownloadSnapshot::new();
    if let Some(dir) = download_dir {
        for entry in fs::read_dir(dir).with_context(|| format!("Failed to read download directory {:?}", dir))? {
            let path = entry?.path();
            if is_report_file(&path) {
                let metadata = fs::metadata(&path)?;
                snapshot.insert(path, (metadata.len(), metadata.modified().ok()));
            }
        }
    }
    Ok(snapshot)
}

fn is_report_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()).map(str::to_ascii_lowercase).as_deref(),
        Some("xlsx" | "xls" | "csv")
    )
}

fn report_file_changed(path: &Path, snapshot: &DownloadSnapshot) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    let current = (metadata.len(), metadata.modified().ok());
    snapshot.get(path).map(|old| old != &current).unwrap_or(true)
}

/// Waits until this export creates or changes a spreadsheet file and the file remains stable.
pub fn wait_for_download(
    download_dir: Option<&PathBuf>,
    timeout_minutes: u64,
    snapshot: &DownloadSnapshot,
) -> Result<()> {
    let Some(dir) = download_dir else {
        return Ok(());
    };

    info!("Waiting for a new or updated report download in {:?}...", dir);
    let timeout = Duration::from_secs(timeout_minutes.saturating_mul(60));
    let start = std::time::Instant::now();
    let mut last_candidate: Option<(PathBuf, u64, Option<SystemTime>)> = None;
    let mut stable_polls = 0u8;

    while start.elapsed() < timeout {
        let entries = fs::read_dir(dir)
            .with_context(|| format!("Failed to read download directory {:?}", dir))?;
        let mut incomplete = false;
        let mut candidate = None;

        for entry in entries {
            let path = entry?.path();
            if matches!(
                path.extension().and_then(|ext| ext.to_str()).map(str::to_ascii_lowercase).as_deref(),
                Some("crdownload" | "tmp")
            ) {
                incomplete = true;
            } else if is_report_file(&path) && report_file_changed(&path, snapshot) {
                let metadata = fs::metadata(&path)?;
                candidate = Some((path, metadata.len(), metadata.modified().ok()));
            }
        }

        if let Some(current) = candidate {
            if !incomplete && last_candidate.as_ref() == Some(&current) {
                stable_polls = stable_polls.saturating_add(1);
                if stable_polls >= 3 {
                    info!("Report download completed: {:?}", current.0);
                    return Ok(());
                }
            } else {
                stable_polls = 0;
            }
            last_candidate = Some(current);
        } else {
            last_candidate = None;
            stable_polls = 0;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    bail!("Timed out waiting for a new or updated report download in {:?}", dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn existing_report_is_not_mistaken_for_current_download() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("report.csv");
        fs::write(&path, b"old report").unwrap();
        let snapshot = snapshot_downloads(Some(&dir.path().to_path_buf())).unwrap();
        assert!(!report_file_changed(&path, &snapshot));
    }

    #[test]
    fn new_report_is_detected() {
        let dir = tempdir().unwrap();
        let snapshot = snapshot_downloads(Some(&dir.path().to_path_buf())).unwrap();
        let path = dir.path().join("report.csv");
        fs::write(&path, b"new report").unwrap();
        assert!(report_file_changed(&path, &snapshot));
    }
}
