use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use std::path::{Path, PathBuf};
use tracing::info;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub dt: NaiveDateTime,
}

pub fn discover_new_files(
    download_dir_path: &Path,
    process_from: Option<NaiveDateTime>,
) -> Vec<FileInfo> {
    let mut new_files = Vec::new();

    for entry in WalkDir::new(download_dir_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let fname = entry.file_name().to_string_lossy().to_string();
        if fname.starts_with("~$") {
            continue;
        }
        if !fname.to_lowercase().contains("average patients seen") {
            continue;
        }
        let ext = entry
            .path()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !["xlsx", "xlsm", "xlsb", "xls"].contains(&ext.as_str()) {
            continue;
        }

        // Extract date time from name: DD-MM-YYYY_HHMMSS
        // Strip extension safely
        let base = if let Some(idx) = fname.rfind('.') {
            &fname[..idx]
        } else {
            &fname
        };

        let parts: Vec<&str> = base.split('_').collect();
        if parts.len() >= 2 {
            let date_str = parts[parts.len() - 2].trim();
            let time_str = parts[parts.len() - 1]
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>();
            if time_str.len() == 6 {
                if let Ok(d) = NaiveDate::parse_from_str(date_str, "%d-%m-%Y") {
                    let hr: u32 = time_str[0..2].parse().unwrap_or(0);
                    let min: u32 = time_str[2..4].parse().unwrap_or(0);
                    let sec: u32 = time_str[4..6].parse().unwrap_or(0);
                    if let Some(t) = NaiveTime::from_hms_opt(hr, min, sec) {
                        let dt = d.and_time(t);
                        if let Some(pf) = process_from {
                            if dt >= pf {
                                new_files.push(FileInfo {
                                    path: entry.path().to_path_buf(),
                                    dt,
                                });
                            }
                        } else {
                            new_files.push(FileInfo {
                                path: entry.path().to_path_buf(),
                                dt,
                            });
                        }
                    }
                }
            }
        }
    }

    new_files.sort_by_key(|f| f.dt);
    info!("Found {} new files to process", new_files.len());

    new_files
}
