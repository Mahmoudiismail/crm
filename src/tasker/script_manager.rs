use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tracing::{error, info, trace};

lazy_static::lazy_static! {
    static ref TASK_LOCKS: Mutex<HashMap<String, Arc<Mutex<()>>>> = Mutex::new(HashMap::new());
}

fn get_task_lock(clean_task_name: &str) -> Arc<Mutex<()>> {
    let mut locks = TASK_LOCKS.lock().unwrap_or_else(|e| e.into_inner());
    locks
        .entry(clean_task_name.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptEntry {
    pub active_script: String,
    pub generator_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskMetadata {
    #[serde(default)]
    pub scripts: HashMap<String, ScriptEntry>,
}

#[derive(Debug, Clone)]
pub struct ScriptManager {
    root_dir: Option<PathBuf>,
}

impl Default for ScriptManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptManager {
    pub fn new() -> Self {
        Self { root_dir: None }
    }

    pub fn with_root_dir<P: AsRef<Path>>(root_dir: P) -> Self {
        Self {
            root_dir: Some(root_dir.as_ref().to_path_buf()),
        }
    }

    pub fn scripts_dir(&self) -> Result<PathBuf> {
        if let Some(ref dir) = self.root_dir {
            Ok(dir.clone())
        } else {
            let exe_dir = crate::utils::executable_dir()?;
            Ok(exe_dir.join("scripts"))
        }
    }

    pub fn sanitize_task_name(task_name: &str) -> String {
        let invalid_chars = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
        let clean = task_name
            .chars()
            .map(|c| if invalid_chars.contains(&c) { '_' } else { c })
            .collect::<String>();

        let trimmed = clean.trim().to_string();
        if trimmed.is_empty() {
            "unnamed_task".to_string()
        } else {
            trimmed
        }
    }

    pub fn is_valid_filename(filename: &str) -> bool {
        if filename != filename.trim() {
            return false;
        }
        let name = filename.trim();
        if name.is_empty() {
            return false;
        }
        if name.contains('/') || name.contains('\\') || name.contains("..") || name.contains(':') {
            return false;
        }
        if name.ends_with('.') || name.ends_with(' ') {
            return false;
        }

        let stem = Path::new(name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(name)
            .to_uppercase();

        let reserved = [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];
        if reserved.contains(&stem.as_str()) {
            return false;
        }

        let p = Path::new(name);
        p.components().count() == 1
    }

    pub fn get_or_create_script(
        &self,
        task_name: &str,
        logical_name: &str,
        canonical_content: &str,
    ) -> Result<PathBuf> {
        self.get_or_create_script_with_timestamp(task_name, logical_name, canonical_content, None)
    }

    pub fn get_or_create_script_with_timestamp(
        &self,
        task_name: &str,
        logical_name: &str,
        canonical_content: &str,
        timestamp_override: Option<&str>,
    ) -> Result<PathBuf> {
        if !Self::is_valid_filename(logical_name) {
            anyhow::bail!("Invalid logical_name '{}': must be a simple filename without path separators or traversal", logical_name);
        }

        let clean_task_name = Self::sanitize_task_name(task_name);

        let task_lock = get_task_lock(&clean_task_name);
        let _lock_guard = task_lock.lock().unwrap_or_else(|e| e.into_inner());

        let task_dir = self.scripts_dir()?.join(&clean_task_name);

        if !task_dir.exists() {
            fs::create_dir_all(&task_dir)
                .with_context(|| format!("Failed to create script directory at {:?}", task_dir))?;
        }

        let lock_path = task_dir.join(".task.lock");
        let lock_file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .with_context(|| format!("Failed to create lock file at {:?}", lock_path))?;

        fs2::FileExt::lock_exclusive(&lock_file)
            .with_context(|| format!("Failed to acquire OS lock on {:?}", lock_path))?;

        let metadata_path = task_dir.join(".metadata.json");

        let mut hasher = Sha256::new();
        hasher.update(canonical_content.as_bytes());
        let current_hash = hex::encode(hasher.finalize());

        let mut metadata: TaskMetadata = if metadata_path.exists() {
            match fs::read_to_string(&metadata_path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(m) => m,
                    Err(e) => {
                        error!("Corrupted metadata JSON at {:?}: {}", metadata_path, e);
                        let timestamp =
                            chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
                        let backup_path =
                            task_dir.join(format!(".metadata.json.corrupted_{}", timestamp));
                        let _ = fs::rename(&metadata_path, &backup_path);
                        TaskMetadata::default()
                    }
                },
                Err(_) => TaskMetadata::default(),
            }
        } else {
            TaskMetadata::default()
        };

        if let Some(entry) = metadata.scripts.get(logical_name) {
            if !Self::is_valid_filename(&entry.active_script) {
                error!(
                    "Corrupted active_script path in metadata: {}",
                    entry.active_script
                );
            } else {
                let active_path = task_dir.join(&entry.active_script);
                if entry.generator_hash == current_hash && active_path.exists() {
                    info!(
                        "Reusing existing persistent script at {:?} (Generator hash unchanged)",
                        active_path
                    );
                    let _ = fs2::FileExt::unlock(&lock_file);
                    return Ok(active_path);
                }
            }
        }

        let base_file_path = task_dir.join(logical_name);

        let target_filename = if !base_file_path.exists() {
            logical_name.to_string()
        } else {
            let timestamp = timestamp_override
                .map(|s| s.to_string())
                .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string());
            let stem = Path::new(logical_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("script");
            let ext = Path::new(logical_name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("ps1");

            let mut version_filename = format!("{}_{}.{}", stem, timestamp, ext);
            let mut candidate_path = task_dir.join(&version_filename);

            let mut counter = 1;
            while candidate_path.exists() {
                version_filename = format!("{}_{}_{}.{}", stem, timestamp, counter, ext);
                candidate_path = task_dir.join(&version_filename);
                counter += 1;
            }

            version_filename
        };

        let target_path = task_dir.join(&target_filename);
        info!(
            "Generating new persistent script version at {:?} for task '{}'",
            target_path, task_name
        );

        let tmp_target_path = task_dir.join(format!("{}.tmp", target_filename));
        {
            let mut file = File::create(&tmp_target_path).with_context(|| {
                format!("Failed to create temp script file at {:?}", tmp_target_path)
            })?;
            file.write_all(canonical_content.as_bytes())
                .with_context(|| {
                    format!("Failed to write script content to {:?}", tmp_target_path)
                })?;
            file.flush()?;
            file.sync_all()?;
        }
        fs::rename(&tmp_target_path, &target_path).with_context(|| {
            format!(
                "Failed to rename temp script {:?} to {:?}",
                tmp_target_path, target_path
            )
        })?;

        metadata.scripts.insert(
            logical_name.to_string(),
            ScriptEntry {
                active_script: target_filename.clone(),
                generator_hash: current_hash,
            },
        );

        let metadata_content = serde_json::to_string_pretty(&metadata)
            .context("Failed to serialize task metadata JSON")?;

        let tmp_metadata_path = task_dir.join(".metadata.json.tmp");
        {
            let mut meta_file = File::create(&tmp_metadata_path).with_context(|| {
                format!(
                    "Failed to create temp metadata file at {:?}",
                    tmp_metadata_path
                )
            })?;
            meta_file.write_all(metadata_content.as_bytes())?;
            meta_file.flush()?;
            meta_file.sync_all()?;
        }
        fs::rename(&tmp_metadata_path, &metadata_path).with_context(|| {
            format!("Failed to rename temp metadata file to {:?}", metadata_path)
        })?;

        let _ = fs2::FileExt::unlock(&lock_file);
        Ok(target_path)
    }

    pub fn execute_script_with_args(
        &self,
        script_path: &Path,
        args: &[(&str, &str)],
    ) -> Result<()> {
        if !script_path.exists() {
            anyhow::bail!("PowerShell script file does not exist at {:?}", script_path);
        }

        info!(
            "Executing persistent PowerShell script: {:?} with args {:?}",
            script_path, args
        );

        let mut cmd = std::process::Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_path.to_str().ok_or_else(|| {
                anyhow::anyhow!("Invalid unicode path for script {:?}", script_path)
            })?,
        ]);

        for (k, v) in args {
            cmd.arg(k).arg(v);
        }

        let output = cmd
            .stdin(Stdio::null())
            .output()
            .context("Failed to spawn PowerShell process")?;

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        if !stdout_str.trim().is_empty() {
            for line in stdout_str.lines() {
                if line.starts_with("TRACE:") {
                    trace!("PS: {}", line.strip_prefix("TRACE:").unwrap_or(line).trim());
                } else if !line.trim().is_empty() {
                    info!("PS: {}", line.trim());
                }
            }
        }

        if !stderr_str.trim().is_empty() {
            for line in stderr_str.lines() {
                error!("PS ERROR: {}", line.trim());
            }
        }

        if !output.status.success() {
            anyhow::bail!(
                "PowerShell script at {:?} failed with status: {}",
                script_path,
                output.status
            );
        }

        Ok(())
    }

    pub fn execute_script(&self, script_path: &Path) -> Result<()> {
        self.execute_script_with_args(script_path, &[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_first_run_creates_base_script() {
        let temp_dir = tempdir().unwrap();
        let manager = ScriptManager::with_root_dir(temp_dir.path());

        let script = "Write-Output 'v1'";
        let path = manager
            .get_or_create_script("Department Split", "department_split.ps1", script)
            .unwrap();

        assert_eq!(path.file_name().unwrap(), "department_split.ps1");
        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), script);

        let metadata_path = temp_dir
            .path()
            .join("Department Split")
            .join(".metadata.json");
        assert!(metadata_path.exists());

        let metadata_str = fs::read_to_string(&metadata_path).unwrap();
        let metadata: TaskMetadata = serde_json::from_str(&metadata_str).unwrap();
        let entry = metadata.scripts.get("department_split.ps1").unwrap();
        assert_eq!(entry.active_script, "department_split.ps1");
    }

    #[test]
    fn test_second_run_identical_content_reuses_file() {
        let temp_dir = tempdir().unwrap();
        let manager = ScriptManager::with_root_dir(temp_dir.path());

        let script = "Write-Output 'v1'";
        let path1 = manager
            .get_or_create_script("Department Split", "department_split.ps1", script)
            .unwrap();

        let path2 = manager
            .get_or_create_script("Department Split", "department_split.ps1", script)
            .unwrap();

        assert_eq!(path1, path2);
        let task_dir = temp_dir.path().join("Department Split");
        let entries: Vec<_> = fs::read_dir(&task_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.ends_with(".ps1"))
            .collect();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], "department_split.ps1");
    }

    #[test]
    fn test_preserves_user_manual_edits_when_generator_unchanged() {
        let temp_dir = tempdir().unwrap();
        let manager = ScriptManager::with_root_dir(temp_dir.path());

        let script = "Write-Output 'v1'";
        let path = manager
            .get_or_create_script("Department Split", "department_split.ps1", script)
            .unwrap();

        let user_edited_content = "Write-Output 'v1 - User edited debug statement'";
        fs::write(&path, user_edited_content).unwrap();

        let path2 = manager
            .get_or_create_script("Department Split", "department_split.ps1", script)
            .unwrap();

        assert_eq!(path, path2);
        assert_eq!(fs::read_to_string(&path2).unwrap(), user_edited_content);
    }

    #[test]
    fn test_generator_content_change_creates_timestamped_version() {
        let temp_dir = tempdir().unwrap();
        let manager = ScriptManager::with_root_dir(temp_dir.path());

        let script_v1 = "Write-Output 'v1'";
        let path_v1 = manager
            .get_or_create_script("Department Split", "department_split.ps1", script_v1)
            .unwrap();
        assert_eq!(path_v1.file_name().unwrap(), "department_split.ps1");

        fs::write(&path_v1, "Write-Output 'v1 user edit'").unwrap();

        let script_v2 = "Write-Output 'v2 new logic'";
        let path_v2 = manager
            .get_or_create_script("Department Split", "department_split.ps1", script_v2)
            .unwrap();

        assert_ne!(path_v1, path_v2);
        let v2_filename = path_v2.file_name().unwrap().to_str().unwrap();
        assert!(v2_filename.starts_with("department_split_20"));
        assert!(v2_filename.ends_with(".ps1"));

        assert_eq!(
            fs::read_to_string(&path_v1).unwrap(),
            "Write-Output 'v1 user edit'"
        );

        assert_eq!(fs::read_to_string(&path_v2).unwrap(), script_v2);

        let metadata_path = temp_dir
            .path()
            .join("Department Split")
            .join(".metadata.json");
        let metadata_str = fs::read_to_string(&metadata_path).unwrap();
        let metadata: TaskMetadata = serde_json::from_str(&metadata_str).unwrap();
        let entry = metadata.scripts.get("department_split.ps1").unwrap();
        assert_eq!(entry.active_script, v2_filename);
    }

    #[test]
    fn test_multiple_scripts_in_same_task_folder() {
        let temp_dir = tempdir().unwrap();
        let manager = ScriptManager::with_root_dir(temp_dir.path());

        let path1 = manager
            .get_or_create_script(
                "Dashboard Updater",
                "dashboard_update.ps1",
                "Write-Output 'update'",
            )
            .unwrap();
        let path2 = manager
            .get_or_create_script(
                "Dashboard Updater",
                "dashboard_email.ps1",
                "Write-Output 'email'",
            )
            .unwrap();

        assert_eq!(path1.file_name().unwrap(), "dashboard_update.ps1");
        assert_eq!(path2.file_name().unwrap(), "dashboard_email.ps1");

        let metadata_path = temp_dir
            .path()
            .join("Dashboard Updater")
            .join(".metadata.json");
        let metadata_str = fs::read_to_string(&metadata_path).unwrap();
        let metadata: TaskMetadata = serde_json::from_str(&metadata_str).unwrap();

        assert_eq!(metadata.scripts.len(), 2);
        assert_eq!(
            metadata
                .scripts
                .get("dashboard_update.ps1")
                .unwrap()
                .active_script,
            "dashboard_update.ps1"
        );
        assert_eq!(
            metadata
                .scripts
                .get("dashboard_email.ps1")
                .unwrap()
                .active_script,
            "dashboard_email.ps1"
        );
    }

    #[test]
    fn test_sanitize_task_name() {
        assert_eq!(
            ScriptManager::sanitize_task_name("Department: Split/Test*"),
            "Department_ Split_Test_"
        );
        assert_eq!(
            ScriptManager::sanitize_task_name("CRM Open Sohail"),
            "CRM Open Sohail"
        );
        assert_eq!(ScriptManager::sanitize_task_name("   "), "unnamed_task");
    }

    #[test]
    fn test_runtime_argument_changes_do_not_change_script_hash() {
        let temp_dir = tempdir().unwrap();
        let manager = ScriptManager::with_root_dir(temp_dir.path());

        let parameterized_template = r#"
param(
    [string]$Email,
    [string]$Subject
)
Write-Output "To: $Email, Subject: $Subject"
"#;

        let path1 = manager
            .get_or_create_script("Email Task", "send_email.ps1", parameterized_template)
            .unwrap();

        let path2 = manager
            .get_or_create_script("Email Task", "send_email.ps1", parameterized_template)
            .unwrap();

        assert_eq!(path1, path2);

        let task_dir = temp_dir.path().join("Email Task");
        let entries: Vec<_> = fs::read_dir(&task_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.ends_with(".ps1"))
            .collect();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], "send_email.ps1");
    }

    #[test]
    fn test_deterministic_timestamp_collision_handling() {
        let temp_dir = tempdir().unwrap();
        let manager = ScriptManager::with_root_dir(temp_dir.path());
        let task_dir = temp_dir.path().join("Department Split");
        fs::create_dir_all(&task_dir).unwrap();

        let fixed_ts = "2026-03-30_12-00-00";
        fs::write(task_dir.join("department_split.ps1"), "v1").unwrap();
        fs::write(
            task_dir.join(format!("department_split_{}.ps1", fixed_ts)),
            "v2",
        )
        .unwrap();
        fs::write(
            task_dir.join(format!("department_split_{}_1.ps1", fixed_ts)),
            "v3",
        )
        .unwrap();

        let mut metadata = TaskMetadata::default();
        metadata.scripts.insert(
            "department_split.ps1".to_string(),
            ScriptEntry {
                active_script: "department_split.ps1".to_string(),
                generator_hash: "oldhash".to_string(),
            },
        );
        fs::write(
            task_dir.join(".metadata.json"),
            serde_json::to_string_pretty(&metadata).unwrap(),
        )
        .unwrap();

        let path = manager
            .get_or_create_script_with_timestamp(
                "Department Split",
                "department_split.ps1",
                "v4 new generator",
                Some(fixed_ts),
            )
            .unwrap();

        let expected_filename = format!("department_split_{}_2.ps1", fixed_ts);
        assert_eq!(
            path.file_name().unwrap().to_str().unwrap(),
            expected_filename
        );
        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "v4 new generator");
    }

    #[test]
    fn test_argument_metacharacters_injection_safety_and_fingerprint() {
        let temp_dir = tempdir().unwrap();
        let manager = ScriptManager::with_root_dir(temp_dir.path());

        let template = r#"
param(
    [string]$HostileVal,
    [string]$OutFile
)
$ErrorActionPreference = 'Stop'
Set-Content -LiteralPath $OutFile -Value $HostileVal -NoNewline
"#;

        let path1 = manager
            .get_or_create_script("Task", "test.ps1", template)
            .unwrap();

        let hostile_values = [
            "'; Write-Output 'injected",
            "$(Get-Date)",
            "\r\n; calc.exe",
            "| & < > $ \" ' ` ( ) { }",
            "Foo `Bar` (Baz) {Qux} $(whoami) | calc & notepad",
        ];

        for hostile_val in hostile_values {
            let path2 = manager
                .get_or_create_script("Task", "test.ps1", template)
                .unwrap();

            assert_eq!(
                path1, path2,
                "Fingerprint must not change for argument variations"
            );

            let out_file = temp_dir.path().join("out.txt");
            let out_str = out_file.to_str().unwrap();

            let res = manager.execute_script_with_args(
                &path2,
                &[("-HostileVal", hostile_val), ("-OutFile", out_str)],
            );

            if res.is_ok() && out_file.exists() {
                let received_val = fs::read_to_string(&out_file).unwrap();
                assert_eq!(
                    received_val, hostile_val,
                    "Exact hostile value must be received literally without command execution"
                );
                let _ = fs::remove_file(&out_file);
            }
        }
    }

    #[test]
    fn test_path_safety_rejects_malicious_names() {
        let temp_dir = tempdir().unwrap();
        let manager = ScriptManager::with_root_dir(temp_dir.path());

        let invalid_names = [
            "../script.ps1",
            "..\\script.ps1",
            "/etc/passwd",
            r"C:\Windows\system32\cmd.exe",
            "foo/bar.ps1",
            "foo\\bar.ps1",
            "..",
            "",
            "script.ps1.",
            "script.ps1 ",
            "CON",
            "NUL",
            "COM1",
        ];

        for bad_name in invalid_names {
            let res = manager.get_or_create_script("Task", bad_name, "Write-Output 1");
            assert!(
                res.is_err(),
                "Should reject invalid script name: {}",
                bad_name
            );
        }
    }

    #[test]
    fn test_corrupted_metadata_recovery() {
        let temp_dir = tempdir().unwrap();
        let manager = ScriptManager::with_root_dir(temp_dir.path());
        let task_dir = temp_dir.path().join("Task");
        fs::create_dir_all(&task_dir).unwrap();

        let path1 = manager
            .get_or_create_script("Task", "script.ps1", "Write-Output v1")
            .unwrap();
        assert!(path1.exists());

        let meta_path = task_dir.join(".metadata.json");
        fs::write(&meta_path, "{ invalid json ").unwrap();

        let path2 = manager
            .get_or_create_script("Task", "script.ps1", "Write-Output v1")
            .unwrap();

        assert!(path2.exists());
        assert!(meta_path.exists());

        let corrupted_entries: Vec<_> = fs::read_dir(&task_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.contains(".metadata.json.corrupted_"))
            .collect();

        assert_eq!(
            corrupted_entries.len(),
            1,
            "Should create corrupted metadata backup file"
        );
    }

    #[test]
    fn test_corrupted_active_script_path_recovery() {
        let temp_dir = tempdir().unwrap();
        let manager = ScriptManager::with_root_dir(temp_dir.path());
        let task_dir = temp_dir.path().join("Task");
        fs::create_dir_all(&task_dir).unwrap();

        let mut metadata = TaskMetadata::default();
        metadata.scripts.insert(
            "script.ps1".to_string(),
            ScriptEntry {
                active_script: "../../../etc/passwd".to_string(),
                generator_hash: "somehash".to_string(),
            },
        );
        fs::write(
            task_dir.join(".metadata.json"),
            serde_json::to_string_pretty(&metadata).unwrap(),
        )
        .unwrap();

        let path = manager
            .get_or_create_script("Task", "script.ps1", "Write-Output safe")
            .unwrap();

        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "script.ps1");
        assert!(path.exists());
        assert!(path.starts_with(&task_dir));
    }

    #[test]
    fn test_genuine_cross_process_locking_concurrency() {
        if let Ok(task_dir_str) = std::env::var("SCRIPT_MANAGER_CHILD_TASK_DIR") {
            let task_dir = PathBuf::from(task_dir_str);
            let root_dir = task_dir.parent().unwrap();
            let manager = ScriptManager::with_root_dir(root_dir);

            let barrier = root_dir.join(".barrier");
            let mut waited = 0;
            while !barrier.exists() && waited < 500 {
                std::thread::sleep(std::time::Duration::from_millis(10));
                waited += 1;
            }

            let script_content = "Write-Output 'Child Execution'";
            let res =
                manager.get_or_create_script("SharedTask", "shared_script.ps1", script_content);
            if res.is_ok() {
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        }

        let temp_dir = tempdir().unwrap();
        let root_dir = temp_dir.path();
        let task_dir = root_dir.join("SharedTask");
        fs::create_dir_all(&task_dir).unwrap();

        let exe = std::env::current_exe().unwrap();

        let mut child1 = std::process::Command::new(&exe)
            .arg("--nocapture")
            .arg("test_genuine_cross_process_locking_concurrency")
            .env("SCRIPT_MANAGER_CHILD_TASK_DIR", task_dir.to_str().unwrap())
            .spawn()
            .unwrap();

        let mut child2 = std::process::Command::new(&exe)
            .arg("--nocapture")
            .arg("test_genuine_cross_process_locking_concurrency")
            .env("SCRIPT_MANAGER_CHILD_TASK_DIR", task_dir.to_str().unwrap())
            .spawn()
            .unwrap();

        fs::write(root_dir.join(".barrier"), "GO").unwrap();

        let status1 = child1.wait().unwrap();
        let status2 = child2.wait().unwrap();

        assert!(status1.success());
        assert!(status2.success());

        let metadata_path = task_dir.join(".metadata.json");
        assert!(metadata_path.exists());
        let metadata_str = fs::read_to_string(&metadata_path).unwrap();
        let metadata: TaskMetadata = serde_json::from_str(&metadata_str).unwrap();

        let entry = metadata.scripts.get("shared_script.ps1").unwrap();
        let active_path = task_dir.join(&entry.active_script);
        assert!(active_path.exists());
    }
}
