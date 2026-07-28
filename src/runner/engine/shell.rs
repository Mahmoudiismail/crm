use crate::runner::engine::logging::TaskLogger;
use crate::runner::engine::process::{run_process, ProcessContext};
use anyhow::Result;

pub async fn run_shell_command(
    logger: &TaskLogger,
    command: &str,
    shell_timeout_seconds: u64,
) -> Result<()> {
    let args = shlex::split(command).ok_or_else(|| {
        anyhow::anyhow!("Failed to parse command string. Ensure quotes are matched.")
    })?;

    if args.is_empty() {
        return Err(anyhow::anyhow!("Command is empty after parsing."));
    }

    let program = &args[0];
    let program_args = &args[1..];

    let mut cmd = tokio::process::Command::new(program);
    cmd.args(program_args);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_shell_command_empty() {
        let logger = TaskLogger::new("test", "test");
        let result = run_shell_command(&logger, "", 5).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Command is empty after parsing."
        );
    }

    #[tokio::test]
    async fn test_run_shell_command_unmatched_quotes() {
        let logger = TaskLogger::new("test", "test");
        let result = run_shell_command(&logger, "echo 'hello", 5).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to parse command string. Ensure quotes are matched."
        );
    }
}
