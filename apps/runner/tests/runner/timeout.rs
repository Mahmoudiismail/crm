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
        command.args([
            "/C",
            "echo child-stdout & echo child-stderr 1>&2 & exit /b 7",
        ]);
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

    let error = result
        .expect_err("non-zero child exit must fail the action")
        .to_string();
    assert!(
        error.contains("exit code Some(7)"),
        "unexpected error: {error}"
    );
    assert!(
        error.contains("child-stdout"),
        "stdout was not preserved: {error}"
    );
    assert!(
        error.contains("child-stderr"),
        "stderr was not preserved: {error}"
    );
}

#[tokio::test]
async fn test_cancelling_execution_terminates_child_process_tree() {
    let logger = TaskLogger::new("cancel_tree_test", "cancel_tree_test");
    let directory = tempfile::tempdir().expect("temporary directory");
    let pid_file = directory.path().join("child.pid");

    #[cfg(target_os = "windows")]
    let cmd = {
        let mut command = tokio::process::Command::new("powershell");
        let script = format!(
            "$child = Start-Process -FilePath powershell -ArgumentList '-NoProfile','-Command','Start-Sleep -Seconds 30' -PassThru; Set-Content -Path '{}' -Value $child.Id; Start-Sleep -Seconds 30",
            pid_file.display()
        );
        command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        command
    };

    #[cfg(unix)]
    let cmd = {
        let mut command = tokio::process::Command::new("sh");
        let script = format!("sleep 30 & echo $! > '{}'; wait", pid_file.display());
        command.args(["-c", &script]);
        command
    };

    let task = tokio::spawn(async move {
        run_process(ProcessContext {
            logger: &logger,
            command_str: "cancel_tree_test".to_string(),
            timeout_seconds: 0,
            cmd,
        })
        .await
    });

    tokio::time::timeout(Duration::from_secs(5), async {
        while !pid_file.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("child process should publish its PID");

    let child_pid = std::fs::read_to_string(&pid_file)
        .expect("read child PID")
        .trim()
        .to_string();
    task.abort();
    let _ = task.await;

    #[cfg(unix)]
    let still_running = {
        tokio::process::Command::new("kill")
            .args(["-0", &child_pid])
            .status()
            .await
            .map(|status| status.success())
            .unwrap_or(false)
    };

    #[cfg(windows)]
    let still_running = {
        let output = tokio::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {child_pid}"), "/NH"])
            .output()
            .await
            .expect("query child process");
        String::from_utf8_lossy(&output.stdout).contains(&child_pid)
    };

    assert!(
        !still_running,
        "cancelling the Runner action must terminate its child process"
    );
}
