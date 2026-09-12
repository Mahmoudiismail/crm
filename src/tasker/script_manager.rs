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
    /// Creates a new `ScriptManager` targeting the standard `scripts/` directory next to the Tasker executable.
    pub fn new() -> Self {
        Self { root_dir: None }
    }

    /// Creates a `ScriptManager` with an explicit root directory (useful for unit testing).
    pub fn with_root_dir<P: AsRef<Path>>(root_dir: P) -> Self {
        Self {
            root_dir: Some(root_dir.as_ref().to_path_buf()),
        }
    }

    /// Resolves the scripts base directory (defaulting to `<exe_dir>/scripts`).
    pub fn scripts_dir(&self) -> Result<PathBuf> {
        if let Some(ref dir) = self.root_dir {
            Ok(dir.clone())
        } else {
            let exe_dir = crate::utils::executable_dir()?;
            Ok(exe_dir.join("scripts"))
        }
    }

    /// Sanitizes a task name to be a safe directory name on Windows filesystem.
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

    /// Returns or creates a persistent PowerShell script file.
    ///
    /// # Behavior
    /// 1. Computes SHA-256 hash of `canonical_content`.
    /// 2. Inspects `.metadata.json` in the task folder.
    /// 3. If `generator_hash` matches, reuses the `active_script` without modifying disk file (preserving manual edits).
    /// 4. If `generator_hash` differs or script does not exist:
    ///    - First run: creates base `<logical_name>` (e.g. `department_split.ps1`).
    ///    - Subsequent Rust generator changes: creates timestamped file `<stem>_YYYY-MM-DD_HH-MM-SS.ps1` (with collision handling `_1`, `_2`).
    ///    - Atomically updates `.metadata.json` and preserves all old versions.
    pub fn get_or_create_script(
        &self,
        task_name: &str,
        logical_name: &str,
        canonical_content: &str,
    ) -> Result<PathBuf> {
        let clean_task_name = Self::sanitize_task_name(task_name);

        // Acquire process-wide task lock to prevent thread races
        let task_lock = get_task_lock(&clean_task_name);
        let _lock_guard = task_lock.lock().unwrap_or_else(|e| e.into_inner());

        let task_dir = self.scripts_dir()?.join(&clean_task_name);

        if !task_dir.exists() {
            fs::create_dir_all(&task_dir)
                .with_context(|| format!("Failed to create script directory at {:?}", task_dir))?;
        }

        let metadata_path = task_dir.join(".metadata.json");

        // Calculate SHA-256 fingerprint of canonical generated content
        let mut hasher = Sha256::new();
        hasher.update(canonical_content.as_bytes());
        let current_hash = hex::encode(hasher.finalize());

        // Read existing metadata if available
        let mut metadata: TaskMetadata = if metadata_path.exists() {
            match fs::read_to_string(&metadata_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => TaskMetadata::default(),
            }
        } else {
            TaskMetadata::default()
        };

        if let Some(entry) = metadata.scripts.get(logical_name) {
            let active_path = task_dir.join(&entry.active_script);
            if entry.generator_hash == current_hash && active_path.exists() {
                info!(
                    "Reusing existing persistent script at {:?} (Generator hash unchanged)",
                    active_path
                );
                return Ok(active_path);
            }
        }

        // Generator hash changed, or script not found on disk
        let base_file_path = task_dir.join(logical_name);

        let target_filename = if !base_file_path.exists() {
            // First time creation: use logical name as base file name
            logical_name.to_string()
        } else {
            // Generator content changed and base file exists: create new timestamped version
            let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
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

        // Write script content securely
        let mut file = File::create(&target_path)
            .with_context(|| format!("Failed to create script file at {:?}", target_path))?;
        file.write_all(canonical_content.as_bytes())
            .with_context(|| format!("Failed to write script content to {:?}", target_path))?;
        file.flush()?;
        file.sync_all()?;
        drop(file);

        // Update metadata atomically
        metadata.scripts.insert(
            logical_name.to_string(),
            ScriptEntry {
                active_script: target_filename.clone(),
                generator_hash: current_hash,
            },
        );

        let metadata_content = serde_json::to_string_pretty(&metadata)
            .context("Failed to serialize task metadata JSON")?;

        crate::utils::atomic_write(&metadata_path, &metadata_content)
            .context("Failed to write script metadata JSON atomically")?;

        Ok(target_path)
    }

    /// Executes a persistent PowerShell script file with CLI parameter arguments and logs stdout/stderr appropriately.
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
                    trace!("PS: {}", line.strip_prefix("TRACE:").unwrap().trim());
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

    /// Executes a persistent PowerShell script file and logs stdout/stderr appropriately.
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

        // User manually edits the file
        let user_edited_content = "Write-Output 'v1 - User edited debug statement'";
        fs::write(&path, user_edited_content).unwrap();

        // Rust code runs again with SAME canonical script content generator
        let path2 = manager
            .get_or_create_script("Department Split", "department_split.ps1", script)
            .unwrap();

        assert_eq!(path, path2);
        // Assert content on disk remains user edited content!
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

        // User edits v1 file
        fs::write(&path_v1, "Write-Output 'v1 user edit'").unwrap();

        // Generator content changes in Rust!
        let script_v2 = "Write-Output 'v2 new logic'";
        let path_v2 = manager
            .get_or_create_script("Department Split", "department_split.ps1", script_v2)
            .unwrap();

        assert_ne!(path_v1, path_v2);
        let v2_filename = path_v2.file_name().unwrap().to_str().unwrap();
        assert!(v2_filename.starts_with("department_split_20"));
        assert!(v2_filename.ends_with(".ps1"));

        // Assert v1 script is untouched
        assert_eq!(
            fs::read_to_string(&path_v1).unwrap(),
            "Write-Output 'v1 user edit'"
        );

        // Assert v2 script has new generator content
        assert_eq!(fs::read_to_string(&path_v2).unwrap(), script_v2);

        // Assert metadata points to v2
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
    fn test_concurrent_script_access() {
        use std::thread;

        let temp_dir = tempdir().unwrap();
        let manager = Arc::new(ScriptManager::with_root_dir(temp_dir.path()));

        let mut handles = vec![];
        for _ in 0..10 {
            let mgr = Arc::clone(&manager);
            handles.push(thread::spawn(move || {
                let script = "Write-Output 'concurrent test'";
                mgr.get_or_create_script("Department Split", "department_split.ps1", script)
                    .unwrap()
            }));
        }

        let mut results = vec![];
        for h in handles {
            results.push(h.join().unwrap());
        }

        let first_path = &results[0];
        assert_eq!(first_path.file_name().unwrap(), "department_split.ps1");
        for p in &results {
            assert_eq!(p, first_path);
        }
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
}
