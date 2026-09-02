use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info};

lazy_static::lazy_static! {
    static ref GLOBAL_DOWNLOAD_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);
}

pub fn get_global_download_dir() -> Arc<Mutex<Option<PathBuf>>> {
    Arc::new(Mutex::new(None)) // To decouple, pass this from caller
}

pub fn configure_download_directory(
    tab: &Arc<headless_chrome::Tab>,
    download_dir: Option<&PathBuf>,
) {
    if let Some(dl_dir) = download_dir {
        info!("Configuring download directory to {:?}", dl_dir);
        if let Err(e) = tab.call_method(headless_chrome::protocol::cdp::Page::SetDownloadBehavior {
            behavior:
                headless_chrome::protocol::cdp::Page::SetDownloadBehaviorBehaviorOption::Allow,
            download_path: Some(dl_dir.to_string_lossy().to_string()),
        }) {
            error!("Failed to set download behavior for tab: {:?}", e);
        }
    }
}

/// Waits for a completed spreadsheet or CSV download in the specified directory.

///

/// A download is considered complete when the directory contains an `.xlsx`, `.xls`,

/// or `.csv` file and no `.crdownload` or `.tmp` files. The wait ends when the

/// download completes or the timeout expires.

///

/// # Examples

///

/// ```

/// use std::fs;

/// use std::path::PathBuf;

///

/// let download_dir = std::env::temp_dir().join("download-example");

/// fs::create_dir_all(&download_dir).unwrap();

/// fs::write(download_dir.join("report.csv"), b"data").unwrap();

///

/// wait_for_download(Some(&download_dir), 1);

///

/// fs::remove_dir_all(download_dir).unwrap();

/// ```

///

/// `download_dir` of `None` leaves the function without waiting.
pub fn wait_for_download(download_dir: Option<&PathBuf>, timeout_minutes: u64) {
    if let Some(dl_dir) = download_dir {
        info!("Waiting for download to complete in {:?}...", dl_dir);
        let mut download_complete = false;
        let timeout_duration = Duration::from_secs(timeout_minutes * 60);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < timeout_duration {
            if let Ok(entries) = std::fs::read_dir(dl_dir) {
                let mut found_incomplete = false;
                let mut found_completed = false;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "crdownload" || ext == "tmp" {
                            found_incomplete = true;
                        } else if ext == "xlsx" || ext == "xls" || ext == "csv" {
                            found_completed = true;
                        }
                    }
                }

                if found_completed && !found_incomplete {
                    download_complete = true;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(200));
        }

        if download_complete {
            info!("Download successfully completed in {:?}", dl_dir);
        } else {
            error!("Download wait timeout or failed in {:?}", dl_dir);
        }
    }
}
