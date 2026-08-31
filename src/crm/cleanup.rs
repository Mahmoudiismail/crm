use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tracing::info;

pub fn cleanup_old_reports(download_dir: &Path, retention_days: u32) -> Result<usize, io::Error> {
    if retention_days == 0 {
        return Ok(0);
    }

    let mut deleted_count = 0;
    let threshold = SystemTime::now() - Duration::from_secs(retention_days as u64 * 86400);

    for entry in fs::read_dir(download_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if (file_name.starts_with("ticket_report_")
                    || file_name.starts_with("lead_report_"))
                    && file_name.ends_with(".csv")
                {
                    let metadata = fs::metadata(&path)?;
                    if let Ok(modified) = metadata.modified() {
                        if modified < threshold {
                            fs::remove_file(&path)?;
                            info!("Deleted old report file: {:?}", path);
                            deleted_count += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(deleted_count)
}
