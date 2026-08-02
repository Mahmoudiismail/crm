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
    let options = FileOptions::<'_, ()>::default().compression_method(CompressionMethod::Deflated);

    let mut bytes_written_to_current_zip: u64 = 0;

    // We roughly estimate the zip size by looking at the uncompressed size.
    // Deflate compresses well, so if the uncompressed size is under 20MB, the compressed is definitely under.
    // If we exceed 20MB of uncompressed size, we'll conservatively rotate.
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

    let mut ps_script = format!(
        r#"
$Outlook = New-Object -ComObject Outlook.Application
$Mail = $Outlook.CreateItem(0)
$Mail.To = "{}"
$Mail.Subject = "logs"
$Mail.Body = "Please find the logs attached."
"#,
        recipient.replace('\"', "'")
    );

    for attachment in attachments {
        ps_script.push_str(&format!(
            "try {{\n    $Mail.Attachments.Add(\"{}\")\n}} catch {{\n    Write-Error \"Failed to attach {:?}\"\n}}\n",
            attachment.display(),
            attachment.display()
        ));
    }

    ps_script.push_str("$Mail.Send()\n");

    crate::tasker::email::outlook::run_powershell(&ps_script)
        .context("Failed to run PowerShell script for sending logs email")?;

    info!("Logs email sent successfully.");
    Ok(())
}
