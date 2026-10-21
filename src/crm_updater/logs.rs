use crate::crm_updater::config::UpdaterConfig;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

struct CollectedFile {
    path: PathBuf,
    relative_path: PathBuf,
}

pub fn process_and_collect_logs(config: &UpdaterConfig) -> Result<()> {
    info!("Starting local report and log collection.");

    let exe_dir = crate::utils::executable_dir()
        .context("Could not determine executable directory for collection")?;
    let configured_logs_dir = Path::new(&config.runner_logs_dir);
    let logs_dir = if configured_logs_dir.is_absolute() {
        configured_logs_dir.to_path_buf()
    } else {
        exe_dir.join(configured_logs_dir)
    };

    let files = collect_files(&exe_dir, &logs_dir);
    if files.is_empty() {
        info!("No report, log, or diagnostic files found. Nothing to move.");
        return Ok(());
    }

    let collection_root = exe_dir.join("collected_logs");
    fs::create_dir_all(&collection_root)?;
    // A new folder per run prevents name collisions and keeps earlier collections intact.
    let run_stamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let mut run_dir = collection_root.join(format!("collection_{}", run_stamp));
    let mut suffix = 1;
    while run_dir.exists() {
        run_dir = collection_root.join(format!("collection_{}_{}", run_stamp, suffix));
        suffix += 1;
    }
    fs::create_dir_all(&run_dir)?;

    let mut moved = 0usize;
    let mut failed = 0usize;
    for file in &files {
        let destination = run_dir.join(&file.relative_path);
        if let Some(parent) = destination.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                warn!("Could not create destination directory {}: {}", parent.display(), error);
                failed += 1;
                continue;
            }
        }

        match move_file(&file.path, &destination) {
            Ok(()) => {
                info!("Moved '{}' to '{}'.", file.path.display(), destination.display());
                moved += 1;
            }
            Err(error) => {
                warn!("Could not move '{}' to '{}': {}", file.path.display(), destination.display(), error);
                failed += 1;
            }
        }
    }

    info!(
        "Local collection finished: moved {} file(s), {} failed. Collection directory: {}",
        moved, failed, run_dir.display()
    );
    if failed > 0 {
        anyhow::bail!("Collection incomplete: moved {} file(s), {} failed; see warnings above", moved, failed);
    }
    Ok(())
}

fn move_file(source: &Path, destination: &Path) -> Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            // Rename can fail when source and destination are on different volumes.
            fs::copy(source, destination).with_context(|| {
                format!("rename failed ({rename_error}); copy fallback also failed")
            })?;
            fs::remove_file(source).with_context(|| {
                format!("copied to {} but could not remove original {}", destination.display(), source.display())
            })?;
            Ok(())
        }
    }
}

fn collect_files(exe_dir: &Path, logs_dir: &Path) -> Vec<CollectedFile> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();

    // Application-level logs beside the updater.
    if let Ok(entries) = fs::read_dir(exe_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && has_extension(&path, &["log"]) {
                add_file(
                    &mut files,
                    &mut seen,
                    path.clone(),
                    PathBuf::from(path.file_name().unwrap_or_default()),
                );
            }
        }
    }

    // Per-task logs under runner_logs_dir.
    if logs_dir.is_dir() {
        let prefix = logs_dir.file_name()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| std::ffi::OsStr::new("logs"));
        for entry in walkdir::WalkDir::new(logs_dir).into_iter().filter_map(|entry| entry.ok()) {
            let path = entry.path();
            if !path.is_file() || !has_extension(path, &["log"]) {
                continue;
            }
            let relative = path.strip_prefix(logs_dir).unwrap_or(path);
            let destination = PathBuf::from(prefix).join(relative);
            add_file(&mut files, &mut seen, path.to_path_buf(), destination);
        }
    } else {
        warn!("Configured Runner logs directory is missing; no task logs were collected.");
    }

    // Yasweb HTML/JSON diagnostics are collected without touching its sent subfolder.
    let diagnostics_dir = exe_dir.join("debug_html");
    if diagnostics_dir.is_dir() {
        for entry in walkdir::WalkDir::new(&diagnostics_dir).into_iter().filter_map(|entry| entry.ok()) {
            let path = entry.path();
            if !path.is_file()
                || !has_extension(path, &["html", "json"])
                || path.starts_with(diagnostics_dir.join("sent"))
            {
                continue;
            }
            let relative = path.strip_prefix(exe_dir).unwrap_or(path);
            add_file(&mut files, &mut seen, path.to_path_buf(), relative.to_path_buf());
        }
    }

    // Reports are written to Downloads folders belonging to the CRM/Yasweb/Tasker apps.
    // Search the common application root, but move only report file types, never executables/configs.
    let search_root = exe_dir.parent().unwrap_or(exe_dir);
    if search_root.is_dir() {
        for entry in walkdir::WalkDir::new(search_root)
            .min_depth(1)
            .max_depth(6)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            let path = entry.path();
            if !entry.file_type().is_dir()
                || !path.file_name().is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("downloads"))
            {
                continue;
            }

            for report_entry in walkdir::WalkDir::new(path).into_iter().filter_map(|entry| entry.ok()) {
                let report_path = report_entry.path();
                if !report_entry.file_type().is_file()
                    || !has_extension(
                        report_path,
                        &["csv", "xls", "xlsx", "xlsm", "pdf", "html", "htm", "json"],
                    )
                {
                    continue;
                }

                let relative = report_path.strip_prefix(search_root).unwrap_or(report_path);
                add_file(&mut files, &mut seen, report_path.to_path_buf(), relative.to_path_buf());
            }
        }
    }

    files
}

fn add_file(
    files: &mut Vec<CollectedFile>,
    seen: &mut HashSet<PathBuf>,
    path: PathBuf,
    relative_path: PathBuf,
) {
    let identity = path.canonicalize().unwrap_or_else(|_| path.clone());
    if seen.insert(identity) {
        if let Err(error) = fs::metadata(&path) {
            tracing::warn!("Skipping unreadable collection candidate {}: {}", path.display(), error);
            return;
        }
        files.push(CollectedFile { path, relative_path });
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
    fn collects_reports_logs_and_diagnostics_without_name_collisions() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let exe_dir = root.join("Updater");
        let logs_dir = root.join("logs");
        let task_dir = logs_dir.join("TaskA");
        let diagnostics_dir = exe_dir.join("debug_html");
        let reports_dir = root.join("crm_windows").join("Downloads");
        fs::create_dir_all(&exe_dir).unwrap();
        fs::create_dir_all(&task_dir).unwrap();
        fs::create_dir_all(&diagnostics_dir).unwrap();
        fs::create_dir_all(&reports_dir).unwrap();
        fs::write(exe_dir.join("crm_updater.log"), "updater log").unwrap();
        fs::write(task_dir.join("same.log"), "task log").unwrap();
        fs::write(diagnostics_dir.join("failure.json"), "{}").unwrap();
        fs::write(reports_dir.join("report.csv"), "a,b").unwrap();
        fs::write(reports_dir.join("ignore.exe"), "not a report").unwrap();

        let files = collect_files(&exe_dir, &logs_dir);
        let names: HashSet<_> = files.iter().map(|file| file.relative_path.clone()).collect();
        assert!(names.contains(&PathBuf::from("crm_updater.log")));
        assert!(names.contains(&PathBuf::from("logs").join("TaskA").join("same.log")));
        assert!(names.contains(&PathBuf::from("debug_html").join("failure.json")));
        assert!(names.contains(&PathBuf::from("crm_windows").join("Downloads").join("report.csv")));
        assert!(!names.contains(&PathBuf::from("crm_windows").join("Downloads").join("ignore.exe")));
        assert_eq!(files.len(), 4);
    }

    #[test]
    fn move_file_removes_source_and_preserves_contents() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("report.csv");
        let destination = temp.path().join("collected_logs").join("report.csv");
        fs::write(&source, "report contents").unwrap();
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        move_file(&source, &destination).unwrap();
        assert!(!source.exists());
        assert_eq!(fs::read_to_string(destination).unwrap(), "report contents");
    }
}
