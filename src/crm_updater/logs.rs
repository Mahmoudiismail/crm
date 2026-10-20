use crate::crm_updater::config::UpdaterConfig;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tracing::info;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

const MAX_ZIP_SIZE_BYTES: u64 = 20 * 1024 * 1024; // 20 MB

struct CollectedFile {
    path: PathBuf,
    archive_name: String,
    delete_after_send: bool,
}

pub fn process_and_collect_logs(config: &UpdaterConfig) -> Result<()> {
    info!("Starting log processing pipeline.");

    let exe_dir = crate::utils::executable_dir()
        .context("Could not determine executable directory for log collection")?;
    let configured_logs_dir = Path::new(&config.runner_logs_dir);
    let logs_dir = if configured_logs_dir.is_absolute() {
        configured_logs_dir.to_path_buf()
    } else {
        exe_dir.join(configured_logs_dir)
    };

    let files = collect_files(&exe_dir, &logs_dir);
    if files.is_empty() {
        info!("No log or diagnostic files found. Skipping email.");
        return Ok(());
    }

    let log_count = files.iter().filter(|file| file.delete_after_send).count();
    let diagnostic_count = files.len() - log_count;
    info!(
        "Found {} log files and {} diagnostic files to process.",
        log_count, diagnostic_count
    );

    // Store archives beside the updater so collection is local and persistent.
    let archive_dir = exe_dir.join("collected_logs");
    fs::create_dir_all(&archive_dir)?;
    let run_stamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let mut zip_archives = Vec::new();
    let mut current_part = 1;
    let mut current_zip_path =
        archive_dir.join(format!("logs_{}_part{}.zip", run_stamp, current_part));
    let mut current_zip = ZipWriter::new(File::create(&current_zip_path)?);
    let options = FileOptions::<()>::default().compression_method(CompressionMethod::Deflated);
    let mut bytes_written_to_current_zip: u64 = 0;

    for file in &files {
        let file_size = fs::metadata(&file.path)?.len();
        if bytes_written_to_current_zip + file_size > MAX_ZIP_SIZE_BYTES
            && bytes_written_to_current_zip > 0
        {
            current_zip.finish()?;
            zip_archives.push(current_zip_path.clone());

            current_part += 1;
            current_zip_path =
                archive_dir.join(format!("logs_{}_part{}.zip", run_stamp, current_part));
            current_zip = ZipWriter::new(File::create(&current_zip_path)?);
            bytes_written_to_current_zip = 0;
        }

        current_zip.start_file(&file.archive_name, options)?;
        let mut source = File::open(&file.path)?;
        let mut buffer = Vec::new();
        source.read_to_end(&mut buffer)?;
        current_zip.write_all(&buffer)?;
        bytes_written_to_current_zip += file_size;
    }

    current_zip.finish()?;
    zip_archives.push(current_zip_path);
    info!(
        "Created {} local log archive(s) in {}. Source files were left unchanged.",
        zip_archives.len(),
        archive_dir.display()
    );

    info!("Local log collection completed successfully.");
    Ok(())
}

fn collect_files(exe_dir: &Path, logs_dir: &Path) -> Vec<CollectedFile> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();

    // Application-level tracing logs are written beside the executable (e.g. crm.log,
    // yasweb.log, and an old wcxx.log if one remains).
    if let Ok(entries) = fs::read_dir(exe_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && has_extension(&path, &["log"]) {
                add_file(
                    &mut files,
                    &mut seen,
                    path.clone(),
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    true,
                );
            }
        }
    }

    // TaskLogger writes per-task logs recursively beneath runner_logs_dir.
    if logs_dir.is_dir() {
        let prefix = logs_dir
            .file_name()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| std::ffi::OsStr::new("logs"))
            .to_string_lossy()
            .into_owned();

        for entry in walkdir::WalkDir::new(logs_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            let path = entry.path();
            if !path.is_file() || !has_extension(path, &["log"]) {
                continue;
            }

            let relative = path.strip_prefix(logs_dir).unwrap_or(path);
            let archive_name = format!(
                "{}/{}",
                prefix,
                relative.to_string_lossy().replace('\\', "/")
            );
            add_file(
                &mut files,
                &mut seen,
                path.to_path_buf(),
                archive_name,
                true,
            );
        }
    } else {
        warn!("Configured Runner logs directory is missing; no task logs were collected.");
    }

    // Yasweb stores HTML snapshots and redacted failure JSON beside the executable.
    let diagnostics_dir = exe_dir.join("debug_html");
    if diagnostics_dir.is_dir() {
        for entry in walkdir::WalkDir::new(&diagnostics_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            let path = entry.path();
            if !path.is_file()
                || !has_extension(path, &["html", "json"])
                || path.starts_with(diagnostics_dir.join("sent"))
            {
                continue;
            }

            let relative = path.strip_prefix(&diagnostics_dir).unwrap_or(path);
            let archive_name = format!(
                "debug_html/{}",
                relative.to_string_lossy().replace('\\', "/")
            );
            add_file(
                &mut files,
                &mut seen,
                path.to_path_buf(),
                archive_name,
                false,
            );
        }
    }

    files
}

fn add_file(
    files: &mut Vec<CollectedFile>,
    seen: &mut HashSet<PathBuf>,
    path: PathBuf,
    archive_name: String,
    delete_after_send: bool,
) {
    let identity = path.canonicalize().unwrap_or_else(|_| path.clone());
    if seen.insert(identity) {
        if let Err(error) = fs::metadata(&path) {
            tracing::warn!(
                "Skipping unreadable collection candidate {}: {}",
                archive_name,
                error
            );
            return;
        }
        files.push(CollectedFile {
            path,
            archive_name,
            delete_after_send,
        });
    }
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extensions
                .iter()
                .any(|expected| extension.eq_ignore_ascii_case(expected))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_application_logs_task_logs_and_diagnostics_without_collisions() {
        let temp = tempfile::tempdir().unwrap();
        let exe_dir = temp.path();
        let logs_dir = exe_dir.join("logs");
        let task_dir = logs_dir.join("TaskA");
        let diagnostics_dir = exe_dir.join("debug_html");
        fs::create_dir_all(&task_dir).unwrap();
        fs::create_dir_all(&diagnostics_dir).unwrap();
        fs::write(exe_dir.join("wcxx.log"), "legacy app log").unwrap();
        fs::write(exe_dir.join("same.log"), "root log").unwrap();
        fs::write(task_dir.join("same.log"), "task log").unwrap();
        fs::write(diagnostics_dir.join("failure.json"), "{}").unwrap();
        fs::write(diagnostics_dir.join("snapshot.html"), "<html></html>").unwrap();
        fs::write(diagnostics_dir.join("ignore.txt"), "not diagnostic").unwrap();

        let files = collect_files(exe_dir, &logs_dir);
        let names: HashSet<_> = files
            .iter()
            .map(|file| file.archive_name.as_str())
            .collect();

        assert!(names.contains("wcxx.log"));
        assert!(names.contains("same.log"));
        assert!(names.contains("logs/TaskA/same.log"));
        assert!(names.contains("debug_html/failure.json"));
        assert!(names.contains("debug_html/snapshot.html"));
        assert!(!names.contains("debug_html/ignore.txt"));
        assert_eq!(files.len(), 5);

        let diagnostic = files
            .iter()
            .find(|file| file.archive_name == "debug_html/failure.json")
            .unwrap();
        assert!(!diagnostic.delete_after_send);
        assert!(files
            .iter()
            .filter(|file| file.archive_name.ends_with(".log"))
            .all(|file| file.delete_after_send));
    }
}
