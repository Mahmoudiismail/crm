use crate::crm_updater::config::UpdaterConfig;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

struct CollectedFile {
    path: PathBuf,
    relative_path: PathBuf,
}

pub fn process_and_collect_logs(config: &UpdaterConfig) -> Result<()> {
    info!("Starting diagnostic log archive collection.");

    let exe_dir = crate::utils::executable_dir()
        .context("Could not determine executable directory for collection")?;
    let configured_logs_dir = Path::new(&config.runner_logs_dir);
    let logs_dir = if configured_logs_dir.is_absolute() {
        configured_logs_dir.to_path_buf()
    } else {
        exe_dir.join(configured_logs_dir)
    };

    let files = collect_files(&exe_dir, &logs_dir, config);
    if files.is_empty() {
        info!("No application log or diagnostic files found. Nothing to archive.");
        return Ok(());
    }

    let collection_root = exe_dir.join("collected_logs");
    fs::create_dir_all(&collection_root)
        .with_context(|| format!("Could not create {}", collection_root.display()))?;
    let run_stamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let mut archive_path = collection_root.join(format!("diagnostics_{}.zip", run_stamp));
    let mut suffix = 1;
    while archive_path.exists() {
        archive_path = collection_root.join(format!("diagnostics_{}_{}.zip", run_stamp, suffix));
        suffix += 1;
    }
    let temp_path = archive_path.with_extension("zip.tmp");

    create_verified_archive(&files, &temp_path).with_context(|| {
        format!(
            "Failed to create diagnostic archive {}",
            archive_path.display()
        )
    })?;
    fs::rename(&temp_path, &archive_path)
        .with_context(|| format!("Could not finalize archive {}", archive_path.display()))?;

    let archive_size = fs::metadata(&archive_path)?.len();
    info!(
        "Diagnostic ZIP created and verified: path={}, files={}, size_bytes={}",
        archive_path.display(),
        files.len(),
        archive_size
    );

    let mut removed = 0usize;
    let mut failed = 0usize;
    for file in &files {
        match fs::remove_file(&file.path) {
            Ok(()) => removed += 1,
            Err(error) => {
                failed += 1;
                warn!(
                    "Archive is safe, but original file could not be removed: path={}, error={}",
                    file.path.display(),
                    error
                );
            }
        }
    }

    info!(
        "Diagnostic archive collection finished: archived={}, originals_removed={}, originals_retained={}, archive={}",
        files.len(), removed, failed, archive_path.display()
    );
    if failed > 0 {
        anyhow::bail!(
            "Diagnostic ZIP was created and verified at {}, but {} original file(s) could not be removed",
            archive_path.display(),
            failed
        );
    }
    Ok(())
}

fn create_verified_archive(files: &[CollectedFile], temp_path: &Path) -> Result<()> {
    let output = File::create(temp_path)
        .with_context(|| format!("Could not create temporary archive {}", temp_path.display()))?;
    let mut writer = ZipWriter::new(output);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for file in files {
        let archive_name = file.relative_path.to_string_lossy().replace('\\', "/");
        let mut input = File::open(&file.path)
            .with_context(|| format!("Could not open {}", file.path.display()))?;
        writer
            .start_file(&archive_name, options)
            .with_context(|| format!("Could not add {} to ZIP", archive_name))?;
        std::io::copy(&mut input, &mut writer)
            .with_context(|| format!("Could not write {} to ZIP", archive_name))?;
    }

    let output = writer.finish().context("Could not finish diagnostic ZIP")?;
    output
        .sync_all()
        .context("Could not flush diagnostic ZIP")?;

    let input = File::open(temp_path)?;
    let mut archive = ZipArchive::new(input).context("Created diagnostic ZIP is invalid")?;
    if archive.len() != files.len() {
        anyhow::bail!(
            "Diagnostic ZIP entry count mismatch: expected {}, found {}",
            files.len(),
            archive.len()
        );
    }

    for file in files {
        let archive_name = file.relative_path.to_string_lossy().replace('\\', "/");
        let mut entry = archive
            .by_name(&archive_name)
            .with_context(|| format!("Verified ZIP is missing {}", archive_name))?;
        let mut archived_bytes = Vec::new();
        entry
            .read_to_end(&mut archived_bytes)
            .with_context(|| format!("Could not read {} from verified ZIP", archive_name))?;
        let source_bytes = fs::read(&file.path)
            .with_context(|| format!("Could not re-read source {}", file.path.display()))?;
        if archived_bytes != source_bytes {
            anyhow::bail!("ZIP content verification failed for {}", archive_name);
        }
    }

    Ok(())
}

fn collect_files(
    exe_dir: &Path,
    legacy_logs_dir: &Path,
    config: &UpdaterConfig,
) -> Vec<CollectedFile> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();

    // Keep collecting Updater logs beside crm_updater.exe.
    collect_root_logs(exe_dir, Path::new("updater"), &mut files, &mut seen);

    // Collect each configured application's root-level logs and its logs/ tree.
    // target_path uses the same base as the updater's replacement logic.
    for (index, app) in config.file_replacement_map.iter().enumerate() {
        let target_dir = resolve_target_dir(exe_dir, &app.target_path);
        if !target_dir.is_dir() {
            warn!(
                "Configured application directory is missing; skipping its logs: app={}, path={}",
                app.executable_name,
                target_dir.display()
            );
            continue;
        }

        let app_name = Path::new(&app.executable_name)
            .file_stem()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("application");
        let archive_prefix = PathBuf::from(format!("applications/{}_{}", app_name, index + 1));

        collect_root_logs(&target_dir, &archive_prefix, &mut files, &mut seen);

        let app_logs_dir = target_dir.join("logs");
        if app_logs_dir.is_dir() {
            collect_log_tree(
                &app_logs_dir,
                &archive_prefix.join("logs"),
                &mut files,
                &mut seen,
            );
        }
    }

    // Backward compatibility: collect the previously configured Runner logs directory.
    if legacy_logs_dir.is_dir() {
        let prefix = legacy_logs_dir
            .file_name()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| std::ffi::OsStr::new("logs"));
        collect_log_tree(
            legacy_logs_dir,
            Path::new("legacy_runner_logs").join(prefix).as_path(),
            &mut files,
            &mut seen,
        );
    } else {
        warn!(
            "Configured legacy Runner logs directory is missing: {}",
            legacy_logs_dir.display()
        );
    }

    // Include HTML/JSON browser diagnostics, but never config files or sent artifacts.
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
                || path.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .to_ascii_lowercase()
                        .contains("config")
                })
            {
                continue;
            }
            let relative = path.strip_prefix(exe_dir).unwrap_or(path);
            add_file(
                &mut files,
                &mut seen,
                path.to_path_buf(),
                relative.to_path_buf(),
            );
        }
    }

    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    files
}

fn resolve_target_dir(exe_dir: &Path, target_path: &str) -> PathBuf {
    let target = Path::new(target_path);
    if target.is_absolute() {
        target.to_path_buf()
    } else {
        exe_dir.join(target)
    }
}

fn collect_root_logs(
    directory: &Path,
    archive_prefix: &Path,
    files: &mut Vec<CollectedFile>,
    seen: &mut HashSet<PathBuf>,
) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && has_extension(&path, &["log"]) {
            let Some(file_name) = path.file_name().map(PathBuf::from) else {
                continue;
            };
            add_file(files, seen, path, archive_prefix.join(file_name));
        }
    }
}

fn collect_log_tree(
    logs_dir: &Path,
    archive_prefix: &Path,
    files: &mut Vec<CollectedFile>,
    seen: &mut HashSet<PathBuf>,
) {
    for entry in walkdir::WalkDir::new(logs_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();
        if !path.is_file() || !has_extension(path, &["log"]) {
            continue;
        }
        let relative = path.strip_prefix(logs_dir).unwrap_or(path);
        add_file(
            files,
            seen,
            path.to_path_buf(),
            archive_prefix.join(relative),
        );
    }
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
            warn!(
                "Skipping unreadable collection candidate {}: {}",
                path.display(),
                error
            );
            return;
        }
        files.push(CollectedFile {
            path,
            relative_path,
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
    use crate::crm_updater::config::{ReplacementMapEntry, UpdaterConfig};

    fn test_config(target_path: &str) -> UpdaterConfig {
        UpdaterConfig {
            downloads_dir: "downloads".into(),
            runner_logs_dir: "logs".into(),
            log_recipient_email: "test@example.com".into(),
            file_replacement_map: vec![ReplacementMapEntry {
                source_file: "runner.exe".into(),
                target_path: target_path.into(),
                executable_name: "runner.exe".into(),
                restart_args: None,
                autostart: true,
            }],
            log_stdout_level: "INFO".into(),
            log_file_level: "INFO".into(),
        }
    }

    #[test]
    fn archive_preserves_layout_and_removes_sources_only_after_verification() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let task_log = root.join("20261031_task.log");
        let yasweb_log = root.join("yasweb.log");
        let html_dir = root.join("debug_html");
        fs::create_dir_all(&html_dir).unwrap();
        let html = html_dir.join("snapshot.html");
        fs::write(&task_log, "runner task log").unwrap();
        fs::write(&yasweb_log, "yasweb log").unwrap();
        fs::write(&html, "<html>diagnostic</html>").unwrap();

        let files = vec![
            CollectedFile {
                path: task_log.clone(),
                relative_path: PathBuf::from("20261031_task.log"),
            },
            CollectedFile {
                path: yasweb_log.clone(),
                relative_path: PathBuf::from("yasweb.log"),
            },
            CollectedFile {
                path: html.clone(),
                relative_path: PathBuf::from("debug_html/snapshot.html"),
            },
        ];
        let archive = root.join("diagnostics.zip");
        create_verified_archive(&files, &archive).unwrap();

        let mut zip = ZipArchive::new(File::open(&archive).unwrap()).unwrap();
        assert!(zip.by_name("20261031_task.log").is_ok());
        assert!(zip.by_name("yasweb.log").is_ok());
        assert!(zip.by_name("debug_html/snapshot.html").is_ok());
        assert!(task_log.exists() && yasweb_log.exists() && html.exists());
    }

    #[test]
    fn collection_excludes_config_files_and_sent_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let exe = temp.path().join("Updater");
        let logs = exe.join("logs");
        let diagnostics = exe.join("debug_html");
        fs::create_dir_all(&logs).unwrap();
        fs::create_dir_all(diagnostics.join("sent")).unwrap();
        let app_dir = exe.join("application");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(app_dir.join("yasweb.log"), "").unwrap();
        fs::write(app_dir.join("yasweb_config.json"), "{}").unwrap();
        fs::write(diagnostics.join("failure.html"), "<html/>").unwrap();
        fs::write(diagnostics.join("config_snapshot.json"), "{}").unwrap();
        fs::write(diagnostics.join("sent").join("sent.html"), "<html/>").unwrap();

        let files = collect_files(&exe, &logs, &test_config("application"));
        let names: HashSet<_> = files
            .iter()
            .map(|file| file.relative_path.clone())
            .collect();
        assert!(names.contains(&PathBuf::from("applications/runner_1/yasweb.log")));
        assert!(names.contains(&PathBuf::from("debug_html/failure.html")));
        assert!(!names
            .iter()
            .any(|name| name.ends_with("yasweb_config.json")));
        assert!(!names
            .iter()
            .any(|name| name.ends_with("config_snapshot.json")));
        assert!(!names.iter().any(|name| name.ends_with("sent.html")));
    }

    #[test]
    fn collection_uses_configured_application_paths_and_keeps_legacy_runner_logs() {
        let temp = tempfile::tempdir().unwrap();
        let updater_dir = temp.path().join("Updater");
        let app_dir = temp.path().join("SeparateApps").join("Runner");
        let app_logs = app_dir.join("logs").join("daily");
        let legacy_logs = temp.path().join("legacy_runner_logs");
        fs::create_dir_all(&updater_dir).unwrap();
        fs::create_dir_all(&app_logs).unwrap();
        fs::create_dir_all(&legacy_logs).unwrap();

        let root_log = app_dir.join("runner.log");
        let nested_log = app_logs.join("task.log");
        let legacy_log = legacy_logs.join("old_task.log");
        fs::write(&root_log, "runner root").unwrap();
        fs::write(&nested_log, "runner task").unwrap();
        fs::write(&legacy_log, "legacy task").unwrap();

        let config = test_config("../SeparateApps/Runner");
        let files = collect_files(&updater_dir, &legacy_logs, &config);
        let names: HashSet<_> = files
            .iter()
            .map(|file| file.relative_path.clone())
            .collect();

        assert!(names.contains(&PathBuf::from("applications/runner_1/runner.log")));
        assert!(names.contains(&PathBuf::from("applications/runner_1/logs/daily/task.log")));
        assert!(names.contains(&PathBuf::from(
            "legacy_runner_logs/legacy_runner_logs/old_task.log"
        )));
    }
}
