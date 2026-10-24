use crate::utils::FileCleanupGuard;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use zip::ZipArchive;

#[derive(Serialize)]
struct AppToStop {
    process_name: String,
    target_paths: Vec<String>,
}

#[derive(Serialize)]
struct FileReplacement {
    source_path: String,
    destination_path: String,
}

#[derive(Serialize)]
struct RestartApp {
    destination_path: String,
    working_directory: String,
    autostart: bool,
    restart_args: Vec<String>,
}

#[derive(Serialize)]
struct UpdaterPayload {
    apps_to_stop: Vec<AppToStop>,
    file_replacements: Vec<FileReplacement>,
    restart_apps: Vec<RestartApp>,
}

pub fn process_update_pipeline(config: &crate::crm_updater::config::UpdaterConfig) -> Result<()> {
    info!("Starting update pipeline.");

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let downloads_dir_buf = exe_dir.join(&config.downloads_dir);
    let downloads_dir = downloads_dir_buf.as_path();

    if !downloads_dir.exists() {
        fs::create_dir_all(downloads_dir)?;
    }

    // 1. Prefer a fresh update ZIP from Outlook Drafts. If Outlook has no draft,
    // allow already-extracted application binaries in downloads/ to be applied.
    let zip_path_opt = download_update_zip_from_drafts(downloads_dir)?;
    let extracted_files = if let Some(zip_path) = zip_path_opt {
        let _zip_guard = FileCleanupGuard::new(&zip_path);
        info!(
            "Found update package {:?}; extracting into {:?}",
            zip_path, downloads_dir
        );
        let files = extract_zip(&zip_path, downloads_dir, b"123456")?;
        if files.is_empty() {
            bail!("Update package was empty; no application files were extracted.");
        }
        files
    } else {
        let existing_sources: Vec<PathBuf> = config
            .file_replacement_map
            .iter()
            .map(|entry| downloads_dir.join(&entry.source_file))
            .filter(|path| path.is_file())
            .collect();
        if existing_sources.is_empty() {
            info!("No update draft or staged application files found. Update pipeline finished.");
            return Ok(());
        }
        info!(
            "No Outlook update draft found; using {} application file(s) already staged in downloads.",
            existing_sources.len()
        );
        existing_sources
    };

    // Never launch a partial update: every configured replacement must have a staged source.
    let missing_sources: Vec<String> = config
        .file_replacement_map
        .iter()
        .map(|entry| downloads_dir.join(&entry.source_file))
        .filter(|path| !path.is_file())
        .map(|path| path.display().to_string())
        .collect();
    if !missing_sources.is_empty() {
        bail!(
            "Update is incomplete; missing {} configured source file(s): {}",
            missing_sources.len(),
            missing_sources.join(", ")
        );
    }

    // Unblock all extracted files
    for file in &extracted_files {
        unblock_file(file);
    }

    // 3. Generate PowerShell script for shutdown, replace, and restart
    let parent_pid = std::process::id();
    let (ps_script_path, args) = generate_update_script(config, downloads_dir, parent_pid)?;

    // Execute script as detached process
    execute_detached_powershell(&ps_script_path, &args)?;

    info!("Update script launched. Exiting crm_updater to allow self-replacement.");
    // Return Ok instead of std::process::exit to ensure destructors (like FileCleanupGuard) run.
    Ok(())
}

#[allow(dead_code)]
const SCAN_DRAFTS_TEMPLATE: &str = include_str!("scripts/scan_update_drafts.ps1");

#[cfg(target_os = "windows")]
fn download_update_zip_from_drafts(downloads_dir: &Path) -> Result<Option<PathBuf>> {
    let abs_downloads_dir = std::fs::canonicalize(downloads_dir)
        .map_err(|e| anyhow::anyhow!("Failed to canonicalize downloads directory: {}", e))?;
    let abs_downloads_dir_str = clean_canonicalized_path(&abs_downloads_dir);
    let result_file = tempfile::Builder::new()
        .prefix("scan_drafts_result_")
        .suffix(".txt")
        .tempfile()?;
    let result_path = result_file.path().to_path_buf();
    drop(result_file);

    let run_result = crate::tasker::script_manager::ScriptManager::new().run_script_with_args(
        "CRM Updater",
        "scan_update_drafts.ps1",
        SCAN_DRAFTS_TEMPLATE,
        &[
            ("-DownloadsDir", abs_downloads_dir_str.as_str()),
            ("-ResultPath", result_path.to_string_lossy().as_ref()),
        ],
    );
    if let Err(error) = run_result {
        let _ = std::fs::remove_file(&result_path);
        return Err(error.context("Failed to scan Outlook drafts through ScriptManager"));
    }

    let result = std::fs::read_to_string(&result_path)
        .map_err(|e| anyhow::anyhow!("Failed to read Outlook draft scan result: {}", e))?;
    let _ = std::fs::remove_file(&result_path);
    let result = result.trim().trim_start_matches('\u{feff}');
    if let Some(file_path) = result.strip_prefix("FOUND:") {
        return Ok(Some(PathBuf::from(file_path.trim())));
    }
    if result == "NOT_FOUND" {
        return Ok(None);
    }
    anyhow::bail!(
        "Outlook draft scan returned an unexpected result: {}",
        result
    )
}

#[cfg(not(target_os = "windows"))]
fn download_update_zip_from_drafts(_downloads_dir: &Path) -> Result<Option<PathBuf>> {
    Ok(None)
}

fn extract_zip(zip_path: &Path, extract_dir: &Path, password: &[u8]) -> Result<Vec<PathBuf>> {
    let file = fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut extracted = Vec::new();

    for i in 0..archive.len() {
        let mut file = match archive.by_index_decrypt(i, password) {
            Ok(f) => f,
            Err(e) => bail!("Failed to decrypt zip file: {:?}", e),
        };

        let outpath = match file.enclosed_name() {
            Some(path) => extract_dir.join(path),
            None => continue,
        };

        if file.name()?.ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
            extracted.push(outpath);
        }
    }

    Ok(extracted)
}

fn unblock_file(path: &Path) {
    let path_string = path.to_string_lossy().into_owned();
    let result = crate::tasker::script_manager::ScriptManager::new().run_script_with_args(
        "CRM Updater",
        "unblock_file.ps1",
        include_str!("scripts/unblock_file.ps1"),
        &[("-LiteralPath", path_string.as_str())],
    );

    if let Err(error) = result {
        warn!(
            "Failed to unblock extracted file {:?} through ScriptManager: {}",
            path, error
        );
    }
}

fn clean_canonicalized_path(path: &Path) -> String {
    let path_str = path.display().to_string();
    path_str
        .strip_prefix(r"\\?\")
        .unwrap_or(&path_str)
        .to_string()
}

fn resolve_target_dir(exe_dir: &Path, target_path: &str) -> PathBuf {
    exe_dir.join(target_path)
}

const UPDATE_SCRIPT_TEMPLATE: &str = include_str!("scripts/apply_update.ps1");

fn generate_update_script(
    config: &crate::crm_updater::config::UpdaterConfig,
    downloads_dir: &Path,
    parent_pid: u32,
) -> Result<(PathBuf, Vec<(String, String)>)> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let log_path = clean_canonicalized_path(&exe_dir.join("updater_detached.log"));
    let abs_downloads_dir = std::fs::canonicalize(downloads_dir)?;
    let downloads_dir_str = clean_canonicalized_path(&abs_downloads_dir);

    let mut apps_to_stop_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    let mut file_replacements = Vec::new();
    let mut restart_apps = Vec::new();

    for entry in &config.file_replacement_map {
        let target_dir = resolve_target_dir(&exe_dir, &entry.target_path);
        let abs_target_dir = if target_dir.exists() {
            std::fs::canonicalize(&target_dir).unwrap_or_else(|_| target_dir.to_path_buf())
        } else {
            target_dir.to_path_buf()
        };
        let abs_target_str = clean_canonicalized_path(&abs_target_dir);
        let dst = Path::new(&abs_target_str).join(&entry.executable_name);

        apps_to_stop_map
            .entry(entry.executable_name.clone())
            .or_default()
            .push(dst.display().to_string());

        let src = Path::new(&downloads_dir_str).join(&entry.source_file);
        file_replacements.push(FileReplacement {
            source_path: src.display().to_string(),
            destination_path: dst.display().to_string(),
        });

        restart_apps.push(RestartApp {
            destination_path: dst.display().to_string(),
            working_directory: abs_target_str,
            autostart: entry.autostart,
            restart_args: entry.restart_args.clone().unwrap_or_default(),
        });
    }

    let mut apps_to_stop: Vec<AppToStop> = apps_to_stop_map
        .into_iter()
        .map(|(app_name, target_paths)| {
            let process_name = app_name
                .strip_suffix(".exe")
                .unwrap_or(&app_name)
                .to_string();
            AppToStop {
                process_name,
                target_paths,
            }
        })
        .collect();

    apps_to_stop.sort_by(|a, b| a.process_name.cmp(&b.process_name));

    let payload = UpdaterPayload {
        apps_to_stop,
        file_replacements,
        restart_apps,
    };

    let payload_json = serde_json::to_string(&payload)?;

    let script_path = crate::tasker::script_manager::ScriptManager::new()
        .get_or_create_script("CRM Updater", "apply_update.ps1", UPDATE_SCRIPT_TEMPLATE)
        .context("Failed to prepare detached updater script through ScriptManager")?;

    let args = vec![
        ("-LogPath".to_string(), log_path),
        ("-DownloadsDir".to_string(), downloads_dir_str),
        ("-ParentPid".to_string(), parent_pid.to_string()),
    ];

    let mut map_file = tempfile::Builder::new()
        .prefix("crm_replacement_map_")
        .suffix(".json")
        .tempfile()?;
    map_file.write_all(payload_json.as_bytes())?;
    let (_, path_buf) = map_file.keep()?; // Detached process handles deletion

    let mut final_args = args;
    final_args.push((
        "-ReplacementMapPath".to_string(),
        path_buf.to_string_lossy().to_string(),
    ));
    Ok((script_path, final_args))
}

fn execute_detached_powershell(script_path: &Path, args: &[(String, String)]) -> Result<()> {
    let borrowed_args: Vec<(&str, &str)> = args
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    crate::tasker::script_manager::ScriptManager::new()
        .execute_script_detached_with_args(script_path, &borrowed_args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_clean_canonicalized_path() {
        let unc_path = Path::new(r"\\?\C:\test\path");
        assert_eq!(clean_canonicalized_path(unc_path), r"C:\test\path");

        let normal_path = Path::new(r"C:\test\path");
        assert_eq!(clean_canonicalized_path(normal_path), r"C:\test\path");

        let unix_path = Path::new(r"/usr/bin/test");
        assert_eq!(clean_canonicalized_path(unix_path), r"/usr/bin/test");
    }

    #[test]
    fn test_resolve_target_dir() {
        let exe_dir = if cfg!(windows) {
            Path::new(r"C:\App")
        } else {
            Path::new("/App")
        };

        assert_eq!(
            resolve_target_dir(exe_dir, "."),
            if cfg!(windows) {
                Path::new(r"C:\App")
            } else {
                Path::new("/App")
            }
        );

        let expected_rel = if cfg!(windows) {
            Path::new(r"C:\App\data\runner")
        } else {
            Path::new("/App/data/runner")
        };
        assert_eq!(
            resolve_target_dir(
                exe_dir,
                if cfg!(windows) {
                    r"data\runner"
                } else {
                    "data/runner"
                }
            ),
            expected_rel
        );

        let abs_path = if cfg!(windows) {
            r"D:\Programs\Runner"
        } else {
            "/usr/bin/runner"
        };
        assert_eq!(resolve_target_dir(exe_dir, abs_path), Path::new(abs_path));
    }

    #[test]
    fn test_generate_update_script_autostart_logic() {
        use crate::crm_updater::config::{ReplacementMapEntry, UpdaterConfig};
        let config = UpdaterConfig {
            downloads_dir: "down".to_string(),
            runner_logs_dir: "logs".to_string(),
            log_recipient_email: "test@test.com".to_string(),
            log_stdout_level: "DEBUG".to_string(),
            log_file_level: "TRACE".to_string(),
            file_replacement_map: vec![
                ReplacementMapEntry {
                    source_file: "src1.exe".to_string(),
                    target_path: ".".to_string(),
                    executable_name: "app1.exe".to_string(),
                    restart_args: None,
                    autostart: true,
                },
                ReplacementMapEntry {
                    source_file: "src2.exe".to_string(),
                    target_path: ".".to_string(),
                    executable_name: "app2.exe".to_string(),
                    restart_args: None,
                    autostart: false,
                },
            ],
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let (script_path, args) = generate_update_script(&config, temp_dir.path(), 99999).unwrap();
        let script_content = std::fs::read_to_string(&script_path).unwrap();

        assert!(script_content.contains("param("));
        assert!(script_content.contains("[string]$ReplacementMapPath"));
        assert!(script_content.contains("ConvertFrom-Json"));

        assert_eq!(args.len(), 4);
        assert_eq!(args[0].0, "-LogPath");
        assert_eq!(args[1].0, "-DownloadsDir");
        assert_eq!(args[2].0, "-ParentPid");
        assert_eq!(args[2].1, "99999");
        assert_eq!(args[3].0, "-ReplacementMapPath");

        let json_val: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&args[3].1).unwrap()).unwrap();
        assert_eq!(json_val["apps_to_stop"][0]["process_name"], "app1");
        assert_eq!(json_val["restart_apps"][0]["autostart"], true);
        assert_eq!(json_val["restart_apps"][1]["autostart"], false);
        std::fs::remove_file(&args[3].1).unwrap();
    }

    #[test]
    fn update_script_preflights_replacements_before_stopping_apps() {
        let script = include_str!("scripts/apply_update.ps1");
        let preflight = script
            .find("# Validate the entire replacement set before stopping any application.")
            .expect("replacement preflight should exist");
        let stop_apps = script
            .find("if ($Config.apps_to_stop)")
            .expect("process-stop phase should exist");
        assert!(
            preflight < stop_apps,
            "all replacement sources and destinations must be checked before stopping apps"
        );
        assert!(script.contains("Required update source file is missing"));
        assert!(script.contains("Destination directory does not exist"));
    }

    #[test]
    fn update_script_stages_verifies_and_rolls_back_partial_replacements() {
        let script = include_str!("scripts/apply_update.ps1");
        assert!(script.contains("$StagedPath = Join-Path"));
        assert!(script.contains("$BackupPath = Join-Path"));
        assert!(script.contains("Staged file verification failed"));
        assert!(script.contains("$ReplacementPhaseComplete = $true"));
        assert!(script.contains("if (-not $ReplacementPhaseComplete -and $AppliedReplacements)"));
        assert!(script.contains("Rollback restored"));
        assert!(script.contains("Destination verification failed"));
        let replacement_phase = script
            .find("# Stage every replacement beside its destination")
            .expect("staging phase should exist");
        let restart_phase = script
            .find("if ($Config.restart_apps)")
            .expect("restart phase should exist");
        assert!(replacement_phase < restart_phase);
    }

    #[test]
    fn test_unblock_file_injection_safety() {
        let src = include_str!("update.rs");
        let bad_pattern = format!("{}{}", "Unblock-File -Path '", "{}'");
        assert!(
            !src.contains(&bad_pattern),
            "Found vulnerable string interpolation in unblock_file"
        );
        let script = include_str!("scripts/unblock_file.ps1");
        assert!(script.contains("[string]$LiteralPath"));
        assert!(script.contains("Unblock-File -LiteralPath $LiteralPath"));

        let malicious_path =
            Path::new("C:\\temp\\file_with 'single quote' & command; calc.exe.txt");
        unblock_file(malicious_path);
    }
}
