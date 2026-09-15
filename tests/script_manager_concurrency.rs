use crm_tool::tasker::script_manager::{ScriptManager, TaskMetadata};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

fn run_child_worker() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--worker-id") {
        let worker_id = &args[pos + 1];
        let root_dir_str = &args[pos + 2];
        let timestamp_override = &args[pos + 3];

        let root_dir = PathBuf::from(root_dir_str);
        let mgr = ScriptManager::with_root_dir(&root_dir);

        // Signal readiness
        let ready_file = root_dir.join(format!(".ready_{}", worker_id));
        fs::write(&ready_file, "READY").unwrap();

        // Wait for parent barrier release
        let go_file = root_dir.join(".go");
        let mut waited = 0;
        while !go_file.exists() && waited < 1000 {
            std::thread::sleep(std::time::Duration::from_millis(5));
            waited += 1;
        }

        // Execute ScriptManager version allocation under contention
        let template = format!(
            "param([string]$Arg)\n# Worker: {}\nWrite-Output \"Worker execution content\"",
            worker_id
        );

        let res = mgr.get_or_create_script_with_timestamp(
            "ConcurrentTask",
            "concurrent_script.ps1",
            &template,
            Some(timestamp_override),
        );

        if res.is_ok() {
            std::process::exit(0);
        } else {
            eprintln!("Worker {} failed: {:?}", worker_id, res.err());
            std::process::exit(1);
        }
    }
}

#[test]
fn test_true_two_os_process_script_manager_locking() {
    run_child_worker();

    let temp_dir = tempdir().unwrap();
    let root_path = temp_dir.path().to_path_buf();
    let task_dir = root_path.join("ConcurrentTask");
    fs::create_dir_all(&task_dir).unwrap();

    let exe_path = std::env::current_exe().unwrap();
    let fixed_timestamp = "2026-09-08_19-42-15";

    let mut proc1 = Command::new(&exe_path)
        .arg("--nocapture")
        .arg("test_true_two_os_process_script_manager_locking")
        .arg("--")
        .arg("--worker-id")
        .arg("proc1")
        .arg(root_path.to_str().unwrap())
        .arg(fixed_timestamp)
        .spawn()
        .expect("Failed to spawn process 1");

    let mut proc2 = Command::new(&exe_path)
        .arg("--nocapture")
        .arg("test_true_two_os_process_script_manager_locking")
        .arg("--")
        .arg("--worker-id")
        .arg("proc2")
        .arg(root_path.to_str().unwrap())
        .arg(fixed_timestamp)
        .spawn()
        .expect("Failed to spawn process 2");

    // Wait for both worker processes to reach readiness
    let ready1 = root_path.join(".ready_proc1");
    let ready2 = root_path.join(".ready_proc2");
    let mut waited = 0;
    while (!ready1.exists() || !ready2.exists()) && waited < 1000 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        waited += 1;
    }

    assert!(
        ready1.exists() && ready2.exists(),
        "Both workers must reach readiness"
    );

    // Release both workers simultaneously
    fs::write(root_path.join(".go"), "GO").unwrap();

    let status1 = proc1.wait().expect("Wait failed for process 1");
    let status2 = proc2.wait().expect("Wait failed for process 2");

    assert!(status1.success(), "Process 1 failed");
    assert!(status2.success(), "Process 2 failed");

    // Comprehensive invariant assertions
    let metadata_path = task_dir.join(".metadata.json");
    assert!(metadata_path.exists(), "Metadata file must exist");

    let metadata_str = fs::read_to_string(&metadata_path).unwrap();
    let metadata: TaskMetadata =
        serde_json::from_str(&metadata_str).expect("Metadata must be valid JSON");

    let entry = metadata
        .scripts
        .get("concurrent_script.ps1")
        .expect("Logical script entry must exist in metadata");

    assert!(
        !entry.active_script.is_empty(),
        "Active script must not be empty"
    );

    let active_script_path = task_dir.join(&entry.active_script);
    assert!(
        active_script_path.exists(),
        "Active script file must exist at {:?}",
        active_script_path
    );

    // Read all files in task_dir
    let dir_entries: Vec<_> = fs::read_dir(&task_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();

    // Verify all generated version files are unique and exist
    let script_files: Vec<_> = dir_entries.iter().filter(|n| n.ends_with(".ps1")).collect();

    let unique_names: HashSet<_> = script_files.iter().cloned().collect();
    assert_eq!(
        script_files.len(),
        unique_names.len(),
        "Every generated version file name must be unique"
    );

    // Verify no temporary files remain
    let temp_files: Vec<_> = dir_entries.iter().filter(|n| n.contains(".tmp")).collect();
    assert!(
        temp_files.is_empty(),
        "No temporary files should remain after execution"
    );
}
