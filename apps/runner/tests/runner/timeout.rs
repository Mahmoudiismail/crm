use runner_app::runner::engine::logging::TaskLogger;
use runner_app::runner::engine::process::{run_process, ProcessContext};
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


#[tokio::test]
async fn test_nonzero_exit_preserves_child_output_and_status() {
    let logger = TaskLogger::new("nonzero_exit_test", "nonzero_exit_test");

    #[cfg(target_os = "windows")]
    let cmd = {
        let mut command = tokio::process::Command::new("cmd");
        command.args(["/C", "echo child-stdout & echo child-stderr 1>&2 & exit /b 7"]);
        command
    };

    #[cfg(not(target_os = "windows"))]
    let cmd = {
        let mut command = tokio::process::Command::new("sh");
        command.args(["-c", "printf child-stdout; printf child-stderr >&2; exit 7"]);
        command
    };

    let result = run_process(ProcessContext {
        logger: &logger,
        command_str: "nonzero_exit_test".to_string(),
        timeout_seconds: 10,
        cmd,
    })
    .await;

    let error = result.expect_err("non-zero child exit must fail the action").to_string();
    assert!(error.contains("exit code Some(7)"), "unexpected error: {error}");
    assert!(error.contains("child-stdout"), "stdout was not preserved: {error}");
    assert!(error.contains("child-stderr"), "stderr was not preserved: {error}");
}
