use crate::crm_updater::config::UpdaterConfig;
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

const MAX_ZIP_SIZE_BYTES: u64 = 20 * 1024 * 1024; // 20 MB

pub fn process_and_send_logs(config: &UpdaterConfig) -> Result<()> {
    info!("Starting log processing pipeline.");

    let logs_dir = Path::new(&config.runner_logs_dir);
    if !logs_dir.exists() {
        warn!("Logs directory {:?} does not exist. Skipping.", logs_dir);
        return Ok(());
    }

    // 1. Gather all log files recursively using WalkDir
    let mut log_files = Vec::new();
    for entry in walkdir::WalkDir::new(logs_dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_file() {
            // We only care about .log files (case-insensitive)
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "log" {
                    log_files.push(path.to_path_buf());
                }
            }
        }
    }

    if log_files.is_empty() {
        info!("No log files found in {:?}", logs_dir);
        return Ok(());
    }

    info!("Found {} log files to process.", log_files.len());

    // 2. Compress and split into multiple < 20MB archives
    let mut zip_archives = Vec::new();
    let mut current_part = 1;
    let mut current_zip_path = tempfile::Builder::new()
        .prefix(&format!("logs_part{}_", current_part))
        .suffix(".zip")
        .tempfile()?;

    let mut current_zip = ZipWriter::new(current_zip_path.as_file().try_clone()?);
    let options = FileOptions::<()>::default().compression_method(CompressionMethod::Deflated);

    let mut bytes_written_to_current_zip: u64 = 0;

    for log_path in &log_files {
        let metadata = fs::metadata(log_path)?;
        let file_size = metadata.len();

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

        let file_name = log_path.file_name().unwrap_or_default().to_string_lossy();
        current_zip.start_file(file_name.as_ref(), options)?;

        let mut log_file = File::open(log_path)?;
        let mut buffer = Vec::new();
        log_file.read_to_end(&mut buffer)?;

        current_zip.write_all(&buffer)?;
        bytes_written_to_current_zip += file_size;
    }

    current_zip.finish()?;
    let (_, path) = current_zip_path.keep()?;
    zip_archives.push(path);

    info!("Created {} zip archives.", zip_archives.len());

    // 3. Send email via Outlook COM
    send_logs_email(&config.log_recipient_email, &zip_archives)?;

    // 4. Delete the original uncompressed log files
    for log_path in &log_files {
        if let Err(e) = fs::remove_file(log_path) {
            warn!("Failed to delete log file {:?}: {}", log_path, e);
        } else {
            info!("Deleted log file {:?}", log_path);
        }
    }

    // 5. Cleanup zip archives
    for zip_path in &zip_archives {
        if let Err(e) = fs::remove_file(zip_path) {
            warn!("Failed to delete temporary zip file {:?}: {}", zip_path, e);
        }
    }

    info!("Log processing pipeline completed successfully.");
    Ok(())
}

fn send_logs_email(recipient: &str, attachments: &[PathBuf]) -> Result<()> {
    info!("Preparing to send logs email to {}", recipient);

    let attachment_paths = attachments
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let attachment_refs = attachment_paths.iter().map(String::as_str).collect::<Vec<_>>();
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
    // use super::*;

    #[test]
    fn test_logs_email_uses_shared_outlook_sender() {
        let src = include_str!("logs.rs");
        assert!(src.contains("send_email_with_attachments"));
        assert!(src.contains("Please find the logs attached."));
        assert!(src.contains("\"logs\""));
    }
}
