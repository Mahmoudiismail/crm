use crate::runner::config::ShellCommandSpec;
use crate::runner::engine::logging::TaskLogger;
use crate::runner::engine::process::{run_process, ProcessContext};
use anyhow::{Context, Result};

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

pub async fn run_shell_sequential(
    logger: &TaskLogger,
    commands: &[ShellCommandSpec],
    shell_timeout_seconds: u64,
) -> Result<()> {
    for spec in commands {
        if let Err(e) = run_shell_command(logger, &spec.command, shell_timeout_seconds).await {
            if !spec.continue_on_error {
                return Err(anyhow::anyhow!("command failed: {}", e));
            }
        }
    }
    Ok(())
}

pub async fn run_shell_parallel(
    logger: &TaskLogger,
    commands: &[ShellCommandSpec],
    shell_timeout_seconds: u64,
) -> Result<()> {
    let handles = commands
        .iter()
        .map(|spec| {
            let spec = spec.clone();
            let l = logger.clone();
            tokio::spawn(async move {
                let result = run_shell_command(&l, &spec.command, shell_timeout_seconds).await;
                (spec, result)
            })
        })
        .collect::<Vec<_>>();

    let mut failures = Vec::new();
    for handle in handles {
        let (spec, result) = handle
            .await
            .context("parallel shell command task join failed")?;
        if let Err(e) = result {
            if !spec.continue_on_error {
                failures.push(format!("{}: {}", spec.command, e));
            }
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "parallel commands failed: {}",
            failures.join("; ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::config::ShellCommandSpec;
    use crate::runner::engine::logging::TaskLogger;

    #[tokio::test]
    async fn sequential_continues_when_command_allows_error() {
        let commands = vec![
            ShellCommandSpec {
                command: "exit 8".to_string(),
                continue_on_error: true,
            },
            ShellCommandSpec {
                command: "echo ok".to_string(),
                continue_on_error: false,
            },
        ];

        run_shell_sequential(&TaskLogger::new("test", "test"), &commands, 5)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn sequential_stops_on_non_continued_error() {
        let commands = vec![ShellCommandSpec {
            command: "exit 8".to_string(),
            continue_on_error: false,
        }];

        assert!(
            run_shell_sequential(&TaskLogger::new("test", "test"), &commands, 5)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn parallel_fails_only_non_continued_errors() {
        let ignored = vec![ShellCommandSpec {
            command: "exit 8".to_string(),
            continue_on_error: true,
        }];
        run_shell_parallel(&TaskLogger::new("test", "test"), &ignored, 5)
            .await
            .unwrap();

        let failed = vec![ShellCommandSpec {
            command: "exit 8".to_string(),
            continue_on_error: false,
        }];
        assert!(
            run_shell_parallel(&TaskLogger::new("test", "test"), &failed, 5)
                .await
                .is_err()
        );
    }
}
