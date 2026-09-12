use crm_tool::runner::engine::logging::TaskLogger;
use crm_tool::runner::engine::process::{run_process, ProcessContext};
use std::time::Duration;

#[tokio::test]
async fn test_process_timeout_terminates_child_and_tree() {
    let logger = TaskLogger::new("timeout_test", "timeout_test");

    #[cfg(target_os = "windows")]
    let cmd = {
        let mut c = tokio::process::Command::new("powershell");
        c.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 10",
        ]);
        c
    };

    #[cfg(not(target_os = "windows"))]
    let cmd = {
        let mut c = tokio::process::Command::new("sh");
        c.args(["-c", "sleep 10 & wait"]);
        c
    };

    let ctx = ProcessContext {
        logger: &logger,
        command_str: "process_tree_timeout_test".to_string(),
        timeout_seconds: 1,
        cmd,
    };

    let start = std::time::Instant::now();
    let res = run_process(ctx).await;
    let elapsed = start.elapsed();

    assert!(
        res.is_err(),
        "Expected process execution to fail with timeout error"
    );
    let err_msg = res.err().unwrap().to_string();
    assert!(
        err_msg.contains("timed out after 1s"),
        "Error message should indicate timeout: {}",
        err_msg
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "Process tree should have been terminated quickly upon timeout, took {:?}",
        elapsed
    );
}
