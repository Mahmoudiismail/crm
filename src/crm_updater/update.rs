use crate::utils::FileCleanupGuard;
use anyhow::{bail, Result};
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

    // 1. Scan Outlook Drafts for the update ZIP
    let zip_path_opt = download_update_zip_from_drafts(downloads_dir)?;

    let zip_path = match zip_path_opt {
        Some(path) => path,
        None => {
            info!("No update draft found. Update pipeline finished.");
            return Ok(());
        }
    };

    let _zip_guard = FileCleanupGuard::new(&zip_path);

    // 2. Extract ZIP
    info!(
        "Extracting update zip {:?} to {:?}",
        zip_path, downloads_dir
    );
    let extracted_files = extract_zip(&zip_path, downloads_dir, b"123456")?;

    if extracted_files.is_empty() {
        warn!("Update zip was empty or extraction failed.");
        return Ok(());
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
const SCAN_DRAFTS_TEMPLATE: &str = r#"
param(
    [string]$DownloadsDir
)

$ErrorActionPreference = 'Stop'
try {
    $Outlook = [Runtime.Interopservices.Marshal]::GetActiveObject("Outlook.Application")
} catch {
    $Outlook = New-Object -ComObject Outlook.Application
}

$Namespace = $Outlook.GetNamespace("MAPI")
$Drafts = $Namespace.GetDefaultFolder(16) # olFolderDrafts

$TargetItem = $null
$TargetAttachment = $null

foreach ($Item in $Drafts.Items) {
    if ($Item.Attachments.Count -gt 0) {
        foreach ($Attachment in $Item.Attachments) {
            if ($Attachment.FileName -match "^crm_tool_.*\.zip$") {
                $TargetItem = $Item
                $TargetAttachment = $Attachment
                break
            }
        }
    }
    if ($TargetItem) { break }
}

if ($TargetItem -and $TargetAttachment) {
    $SavePath = Join-Path $DownloadsDir $TargetAttachment.FileName
    $TargetAttachment.SaveAsFile($SavePath)
    Write-Output "FOUND:$SavePath"
    $TargetItem.Delete()
} else {
    Write-Output "NOT_FOUND"
}
"#;

#[cfg(target_os = "windows")]
fn download_update_zip_from_drafts(downloads_dir: &Path) -> Result<Option<PathBuf>> {
    let abs_downloads_dir = match std::fs::canonicalize(downloads_dir) {
        Ok(path) => path,
        Err(e) => bail!("Failed to canonicalize downloads directory: {}", e),
    };

    let abs_downloads_dir_str = clean_canonicalized_path(&abs_downloads_dir);

    let mut temp_file = tempfile::Builder::new()
        .prefix("scan_drafts_")
        .suffix(".ps1")
        .tempfile()?;

    temp_file.write_all(SCAN_DRAFTS_TEMPLATE.as_bytes())?;
    temp_file.as_file().sync_all()?;

    let (file, path) = temp_file.keep()?;
    drop(file);
    let _guard = FileCleanupGuard::new(&path);

    let output = std::process::Command::new("powershell")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&path)
        .arg("-DownloadsDir")
        .arg(&abs_downloads_dir_str)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("FOUND:") {
            let file_path = line.trim_start_matches("FOUND:");
            return Ok(Some(PathBuf::from(file_path)));
        } else if line == "NOT_FOUND" {
            return Ok(None);
        }
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("PowerShell draft scan failed: {}", stderr);
    }

    Ok(None)
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

        if file.name().ends_with('/') {
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
    let _ = std::process::Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg("param([string]$Path) Unblock-File -LiteralPath $Path")
        .arg(path)
        .status();
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

const UPDATE_SCRIPT_TEMPLATE: &str = r#"
param(
    [string]$LogPath,
    [string]$DownloadsDir,
    [int]$ParentPid,
    [string]$ReplacementMapJson
)

$ErrorActionPreference = 'Stop'

function Write-Log {
    param([string]$Message)
    $Timestamp = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'
    $LogLine = "[$Timestamp] $Message"
    Write-Output $LogLine
    if ($LogPath) {
        Add-Content -LiteralPath $LogPath -Value $LogLine
    }
}

try {
    Write-Log "Detached update process started."
    Write-Log "Downloads directory resolved to: $DownloadsDir"
    Write-Log "Waiting for original updater process (PID: $ParentPid) to exit..."

    $TimeoutSeconds = 30
    $WaitCount = 0
    while ((Get-Process -Id $ParentPid -ErrorAction SilentlyContinue) -and ($WaitCount -lt $TimeoutSeconds)) {
        Start-Sleep -Seconds 1
        $WaitCount++
    }

    if (Get-Process -Id $ParentPid -ErrorAction SilentlyContinue) {
        Write-Log "FAILURE: Original updater process (PID: $ParentPid) failed to terminate after $TimeoutSeconds seconds."
        throw "Original updater termination timeout"
    } else {
        Write-Log "Original updater process terminated successfully."
    }

    $Config = $ReplacementMapJson | ConvertFrom-Json

    if ($Config.apps_to_stop) {
        foreach ($App in $Config.apps_to_stop) {
            $ProcessName = $App.process_name
            $TargetPaths = @($App.target_paths)

            $Processes = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue
            if ($Processes) {
                foreach ($Proc in $Processes) {
                    $ProcPath = $null
                    $PathError = $null

                    try {
                        $ProcPath = $Proc.Path
                    } catch {
                        $PathError = $_
                    }

                    if ([string]::IsNullOrWhiteSpace($ProcPath)) {
                        try {
                            $ProcPath = $Proc.MainModule.FileName
                        } catch {
                            if (-not $PathError) {
                                $PathError = $_
                            } else {
                                $PathError = "$PathError | $_"
                            }
                        }
                    }

                    if ([string]::IsNullOrWhiteSpace($ProcPath)) {
                        Write-Log "FAILURE: Cannot safely inspect process path for process ID $($Proc.Id) ($ProcessName). Error: $PathError"
                        throw "Unsafe process targeting: Cannot inspect process path."
                    }

                    $IsTargetMatch = $false
                    foreach ($tp in $TargetPaths) {
                        if ([string]::Equals($ProcPath.Trim(), $tp.Trim(), [System.StringComparison]::OrdinalIgnoreCase)) {
                            $IsTargetMatch = $true
                            break
                        }
                    }

                    if ($IsTargetMatch) {
                        Write-Log "Target process '$ProcessName' (PID: $($Proc.Id)) matches one of the target paths. Stopping..."

                        try {
                            Stop-Process -Id $Proc.Id -Force -ErrorAction Stop
                        } catch {
                            Write-Log "FAILURE: Stop-Process failed for process '$ProcessName' (PID: $($Proc.Id)). Error: $_"
                            throw "Process termination error"
                        }

                        $WaitCount = 0
                        while ((Get-Process -Id $Proc.Id -ErrorAction SilentlyContinue) -and ($WaitCount -lt $TimeoutSeconds)) {
                            Start-Sleep -Seconds 1
                            $WaitCount++
                        }

                        if (Get-Process -Id $Proc.Id -ErrorAction SilentlyContinue) {
                            Write-Log "FAILURE: Process '$ProcessName' (PID: $($Proc.Id)) failed to terminate after $TimeoutSeconds seconds."
                            throw "Process termination timeout"
                        } else {
                            Write-Log "Process '$ProcessName' (PID: $($Proc.Id)) terminated successfully."
                        }
                    } else {
                        Write-Log "Process '$ProcessName' (PID: $($Proc.Id)) is running at a different path ($ProcPath). Skipping termination."
                    }
                }
            } else {
                Write-Log "Target process '$ProcessName' is not running. No stop required."
            }
        }
    }

    if ($Config.file_replacements) {
        foreach ($Item in $Config.file_replacements) {
            $SrcPath = $Item.source_path
            $DstPath = $Item.destination_path

            if (Test-Path -LiteralPath $SrcPath) {
                Write-Log "Replacing '$DstPath' with '$SrcPath'..."
                Copy-Item -LiteralPath $SrcPath -Destination $DstPath -Force
                if (Test-Path -LiteralPath $DstPath) {
                    $SrcHash = (Get-FileHash -LiteralPath $SrcPath -Algorithm SHA256).Hash
                    $DstHash = (Get-FileHash -LiteralPath $DstPath -Algorithm SHA256).Hash
                    if ($SrcHash -eq $DstHash) {
                        Write-Log "Successfully replaced '$DstPath' and verified SHA-256 hash."
                    } else {
                        Write-Log "FAILURE: Hash mismatch after copying to '$DstPath'. Source: $SrcHash, Dest: $DstHash"
                        throw "File verification failed"
                    }
                } else {
                    Write-Log "FAILURE: File '$DstPath' not found after copy."
                    throw "File copy failed"
                }
            } else {
                Write-Log "Source file '$SrcPath' not found. Skipping replacement."
            }
        }
    }

    if ($Config.restart_apps) {
        foreach ($App in $Config.restart_apps) {
            $DstPath = $App.destination_path
            $WorkDir = $App.working_directory
            $Autostart = $App.autostart
            $RestartArgs = @($App.restart_args)

            if ($Autostart) {
                if (Test-Path -LiteralPath $DstPath) {
                    Write-Log "Autostart is enabled. Starting '$DstPath'..."
                    if ($RestartArgs -and $RestartArgs.Count -gt 0) {
                        Start-Process -FilePath $DstPath -WorkingDirectory $WorkDir -ArgumentList $RestartArgs
                    } else {
                        Start-Process -FilePath $DstPath -WorkingDirectory $WorkDir
                    }
                    Write-Log "Started '$DstPath' successfully."
                }
            } else {
                Write-Log "Autostart is disabled for '$DstPath'. Leaving it stopped."
            }
        }
    }

    if ($Config.file_replacements) {
        foreach ($Item in $Config.file_replacements) {
            $SrcPath = $Item.source_path
            if (Test-Path -LiteralPath $SrcPath) {
                Remove-Item -LiteralPath $SrcPath -Force
                Write-Log "Cleaned up source file '$SrcPath'."
            }
        }
    }

    Write-Log "SUCCESS: Update completed successfully."
} catch {
    Write-Log "FAILURE: An error occurred during the update process: $_"
    $UpdateFailed = $true
} finally {
    Remove-Item -LiteralPath $PSCommandPath -Force -ErrorAction SilentlyContinue
    if ($UpdateFailed) {
        exit 1
    }
}
"#;

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

    let mut temp_file = tempfile::Builder::new()
        .prefix("update_")
        .suffix(".ps1")
        .tempfile()?;

    temp_file.write_all(UPDATE_SCRIPT_TEMPLATE.as_bytes())?;
    temp_file.as_file().sync_all()?;

    let (file, script_path) = temp_file.keep()?;
    drop(file);

    let args = vec![
        ("-LogPath".to_string(), log_path),
        ("-DownloadsDir".to_string(), downloads_dir_str),
        ("-ParentPid".to_string(), parent_pid.to_string()),
        ("-ReplacementMapJson".to_string(), payload_json),
    ];

    Ok((script_path, args))
}

fn execute_detached_powershell(script_path: &Path, args: &[(String, String)]) -> Result<()> {
    let mut cmd = std::process::Command::new("powershell");
    cmd.arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-WindowStyle")
        .arg("Hidden")
        .arg("-File")
        .arg(script_path);

    for (k, v) in args {
        cmd.arg(k).arg(v);
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;

        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }

    cmd.spawn()?;
    Ok(())
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
        assert!(script_content.contains("[string]$ReplacementMapJson"));
        assert!(script_content.contains("ConvertFrom-Json"));

        assert_eq!(args.len(), 4);
        assert_eq!(args[0].0, "-LogPath");
        assert_eq!(args[1].0, "-DownloadsDir");
        assert_eq!(args[2].0, "-ParentPid");
        assert_eq!(args[2].1, "99999");
        assert_eq!(args[3].0, "-ReplacementMapJson");

        let json_val: serde_json::Value = serde_json::from_str(&args[3].1).unwrap();
        assert_eq!(json_val["apps_to_stop"][0]["process_name"], "app1");
        assert_eq!(json_val["restart_apps"][0]["autostart"], true);
        assert_eq!(json_val["restart_apps"][1]["autostart"], false);
    }

    #[test]
    fn test_unblock_file_injection_safety() {
        let src = include_str!("update.rs");
        let bad_pattern = format!("{}{}", "Unblock-File -Path '", "{}'");
        assert!(
            !src.contains(&bad_pattern),
            "Found vulnerable string interpolation in unblock_file"
        );

        let malicious_path =
            Path::new("C:\\temp\\file_with 'single quote' & command; calc.exe.txt");
        unblock_file(malicious_path);
    }
}
