use crate::crm_updater::config::UpdaterConfig;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

const MAX_ZIP_SIZE_BYTES: u64 = 20 * 1024 * 1024; // 20 MB

struct CollectedFile {
    path: PathBuf,
    archive_name: String,
    delete_after_send: bool,
}

pub fn process_and_send_logs(config: &UpdaterConfig) -> Result<()> {
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

    // Split the collected files into multiple archives using their uncompressed size as a
    // conservative estimate. Preserve relative paths so equal file names cannot collide.
    let mut zip_archives = Vec::new();
    let mut current_part = 1;
    let mut current_zip_path = tempfile::Builder::new()
        .prefix(&format!("logs_part{}_", current_part))
        .suffix(".zip")
        .tempfile()?;
    let mut current_zip = ZipWriter::new(current_zip_path.as_file().try_clone()?);
    let options = FileOptions::<()>::default().compression_method(CompressionMethod::Deflated);
    let mut bytes_written_to_current_zip: u64 = 0;

    for file in &files {
        let file_size = fs::metadata(&file.path)?.len();
        if bytes_written_to_current_zip + file_size > MAX_ZIP_SIZE_BYTES
            && bytes_written_to_current_zip > 0
        {
            current_zip.finish()?;
            let (_, path) = current_zip_path.keep()?;
            zip_archives.push(path);

            current_part += 1;
            current_zip_path = tempfile::Builder::new()
                .prefix(&format!("logs_part{}_", current_part))
                .suffix(".zip")
                .tempfile()?;
            current_zip = ZipWriter::new(current_zip_path.as_file().try_clone()?);
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
    let (_, path) = current_zip_path.keep()?;
    zip_archives.push(path);
    info!("Created {} zip archives.", zip_archives.len());

    // Keep originals if sending fails so the next run can retry.
    send_logs_email(&config.log_recipient_email, &zip_archives)?;

    // Delete collected .log files only after successful delivery. Keep HTML/JSON diagnostics
    // on disk because they are useful for investigating a later or recurring failure.
    for file in files.iter().filter(|file| file.delete_after_send) {
        if let Err(e) = fs::remove_file(&file.path) {
            warn!("Failed to delete log file {:?}: {}", file.path, e);
        } else {
            info!("Deleted log file {:?}", file.path);
        }
    }

    for zip_path in &zip_archives {
        if let Err(e) = fs::remove_file(zip_path) {
            warn!("Failed to delete temporary zip file {:?}: {}", zip_path, e);
        }
    }

    info!("Log processing pipeline completed successfully.");
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
                    path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
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
            add_file(&mut files, &mut seen, path.to_path_buf(), archive_name, true);
        }
    } else {
        warn!("Configured logs directory {:?} does not exist.", logs_dir);
    }

    // Yasweb stores HTML snapshots and redacted failure JSON beside the executable.
    let diagnostics_dir = exe_dir.join("debug_html");
    if diagnostics_dir.is_dir() {
        for entry in walkdir::WalkDir::new(&diagnostics_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            let path = entry.path();
            if !path.is_file() || !has_extension(path, &["html", "json"]) {
                continue;
            }

            let relative = path.strip_prefix(&diagnostics_dir).unwrap_or(path);
            let archive_name = format!(
                "debug_html/{}",
                relative.to_string_lossy().replace('\\', "/")
            );
            add_file(&mut files, &mut seen, path.to_path_buf(), archive_name, false);
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

fn send_logs_email(recipient: &str, attachments: &[PathBuf]) -> Result<()> {
    info!("Preparing to send logs email to {}", recipient);

    let attachment_paths = attachments
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let attachment_refs = attachment_paths
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let html_body = "<html><body>Please find the logs attached.</body></html>";

    crate::tasker::email::outlook::send_email_with_attachments(
        recipient,
        "",
        "logs",
        html_body,
        &attachment_refs,
        "Send()",
    )
    .context("Failed to send logs email through Outlook")?;

    info!("Logs email sent successfully.");
    Ok(())
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
        let names: HashSet<_> = files.iter().map(|file| file.archive_name.as_str()).collect();

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

    #[test]
    fn test_logs_email_uses_shared_outlook_sender() {
        let src = include_str!("logs.rs");
        assert!(src.contains("send_email_with_attachments"));
        assert!(src.contains("Please find the logs attached."));
        assert!(src.contains("\"logs\""));
    }
}
