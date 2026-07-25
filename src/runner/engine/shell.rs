use crate::runner::engine::logging::TaskLogger;
use crate::runner::engine::process::{run_process, ProcessContext};
use anyhow::Result;

pub async fn run_shell_command(
    logger: &TaskLogger,
    command: &str,
    shell_timeout_seconds: u64,
) -> Result<()> {
    #[cfg(target_os = "windows")]
    let cmd = {
        let mut c = tokio::process::Command::new("cmd.exe");
        c.arg("/c").arg(command);
        c
    };

    #[cfg(not(target_os = "windows"))]
    let cmd = {
        let mut c = tokio::process::Command::new("bash");
        c.arg("-lc").arg(command);
        c
    };

    let ctx = ProcessContext {
        logger,
        command_str: format!("shell command: {}", command),
        timeout_seconds: shell_timeout_seconds,
        cmd,
    };

    run_process(ctx)
        .await
        .map_err(|e| anyhow::anyhow!("Command failed: {}", e))
}
