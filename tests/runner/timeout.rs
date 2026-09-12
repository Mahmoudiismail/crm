use crm_tool::runner::engine::logging::TaskLogger;
use crm_tool::runner::engine::process::{run_process, ProcessContext};
use std::time::Duration;

#[tokio::test]
async fn test_process_timeout_terminates_child() {
    let logger = TaskLogger::new("timeout_test", "timeout_test");
    let mut cmd = tokio::process::Command::new("sleep");
    cmd.arg("10");

    let ctx = ProcessContext {
        logger: &logger,
        command_str: "sleep 10".to_string(),
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
        "Process should have been terminated quickly upon timeout, took {:?}",
        elapsed
    );
}
