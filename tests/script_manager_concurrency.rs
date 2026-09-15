use crm_tool::tasker::script_manager::ScriptManager;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn run_child_if_args() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--child-process") {
        let root_dir = &args[args.len() - 2];
        let proc_id = &args[args.len() - 1];
        let mgr = ScriptManager::with_root_dir(root_dir);

        for i in 0..10 {
            let template = format!(
                "param([string]$Arg)\nWrite-Output \"Proc: {} Iter: {}\"",
                proc_id, i
            );
            let _ = mgr
                .get_or_create_script("ConcurrentTask", "concurrent_script.ps1", &template)
                .unwrap();
        }
        std::process::exit(0);
    }
}

#[test]
fn test_true_two_os_process_script_manager_locking() {
    run_child_if_args();

    let temp_dir = tempdir().unwrap();
    let root_path = temp_dir.path().to_path_buf();

    let exe_path = std::env::current_exe().unwrap();

    let task_name = "ConcurrentTask";
    let logical_name = "concurrent_script.ps1";

    let handle1 = Command::new(&exe_path)
        .arg("test_true_two_os_process_script_manager_locking")
        .arg("--exact")
        .arg("--nocapture")
        .arg("--")
        .arg("--child-process")
        .arg(root_path.to_str().unwrap())
        .arg("Proc1")
        .spawn()
        .expect("Failed to spawn process 1");

    let handle2 = Command::new(&exe_path)
        .arg("test_true_two_os_process_script_manager_locking")
        .arg("--exact")
        .arg("--nocapture")
        .arg("--")
        .arg("--child-process")
        .arg(root_path.to_str().unwrap())
        .arg("Proc2")
        .spawn()
        .expect("Failed to spawn process 2");

    let out1 = handle1.wait_with_output().unwrap();
    let out2 = handle2.wait_with_output().unwrap();

    assert!(
        out1.status.success(),
        "Process 1 failed with status {}\nStdout: {}\nStderr: {}",
        out1.status,
        String::from_utf8_lossy(&out1.stdout),
        String::from_utf8_lossy(&out1.stderr)
    );

    assert!(
        out2.status.success(),
        "Process 2 failed with status {}\nStdout: {}\nStderr: {}",
        out2.status,
        String::from_utf8_lossy(&out2.stdout),
        String::from_utf8_lossy(&out2.stderr)
    );

    let task_dir = root_path.join(task_name);
    let metadata_path = task_dir.join(".metadata.json");

    assert!(metadata_path.exists(), "Metadata file must exist");
    let metadata_str = fs::read_to_string(&metadata_path).unwrap();
    let metadata: crm_tool::tasker::script_manager::TaskMetadata =
        serde_json::from_str(&metadata_str).expect("Metadata must be valid JSON");

    let entry = metadata
        .scripts
        .get(logical_name)
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

    // Verify all referenced script files exist
    for (name, entry) in &metadata.scripts {
        let path = task_dir.join(&entry.active_script);
        assert!(
            path.exists(),
            "Referenced script file for {} ({}) must exist",
            name,
            entry.active_script
        );
    }
}
