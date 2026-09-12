use anyhow::{Context, Result};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;

use crate::runner::engine::helpers::excerpt_utf8;
use crate::runner::engine::logging::TaskLogger;

#[derive(Debug)]
pub struct ProcessContext<'a> {
    pub logger: &'a TaskLogger,
    pub command_str: String,
    pub timeout_seconds: u64,
    pub cmd: tokio::process::Command,
}

pub async fn run_process(mut ctx: ProcessContext<'_>) -> Result<()> {
    ctx.logger
        .log("--------------------------------------------------")
        .await;
    ctx.logger
        .log(&format!(">>> EXECUTING ACTION: {}", ctx.command_str))
        .await;
    ctx.logger
        .log("--------------------------------------------------")
        .await;

    ctx.cmd.stdout(Stdio::piped());
    ctx.cmd.stderr(Stdio::piped());
    ctx.cmd.kill_on_drop(true);

    let mut child = ctx
        .cmd
        .spawn()
        .with_context(|| format!("Failed to spawn process for command: {}", ctx.command_str))?;

    let child_pid = child.id();

    let timeout_duration = if ctx.timeout_seconds > 0 {
        Some(Duration::from_secs(ctx.timeout_seconds))
    } else {
        None
    };

    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();

    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();

    let mut stdout_stream = stdout_handle;
    let mut stderr_stream = stderr_handle;

    let read_stdout = async {
        if let Some(ref mut stream) = stdout_stream {
            let _ = stream.read_to_end(&mut stdout_bytes).await;
        }
    };

    let read_stderr = async {
        if let Some(ref mut stream) = stderr_stream {
            let _ = stream.read_to_end(&mut stderr_bytes).await;
        }
    };

    let status = if let Some(duration) = timeout_duration {
        let wait_child = async {
            let (status_res, _, _) = tokio::join!(child.wait(), read_stdout, read_stderr);
            status_res
        };

        match tokio::time::timeout(duration, wait_child).await {
            Ok(res) => res.context("Failed waiting for process completion")?,
            Err(_) => {
                ctx.logger
                    .log(&format!(
                        "TIMEOUT: Action exceeded timeout of {}s. Terminating process tree...",
                        ctx.timeout_seconds
                    ))
                    .await;

                terminate_process_tree(child_pid, &mut child).await;

                return Err(anyhow::anyhow!(
                    "Command timed out after {}s: {}",
                    ctx.timeout_seconds,
                    ctx.command_str
                ));
            }
        }
    } else {
        let (status_res, _, _) = tokio::join!(child.wait(), read_stdout, read_stderr);
        status_res.context("Failed waiting for process completion")?
    };

    ctx.logger.log_bytes("STDOUT", &stdout_bytes).await;
    ctx.logger.log_bytes("STDERR", &stderr_bytes).await;

    if !status.success() {
        let stdout_excerpt = excerpt_utf8(&stdout_bytes);
        let stderr_excerpt = excerpt_utf8(&stderr_bytes);
        return Err(anyhow::anyhow!(
            "Command failed with exit code {:?}

STDOUT EXCERPT:
{}

STDERR EXCERPT:
{}",
            status.code(),
            stdout_excerpt,
            stderr_excerpt
        ));
    }

    Ok(())
}

async fn terminate_process_tree(child_pid: Option<u32>, child: &mut tokio::process::Child) {
    if let Some(pid) = child_pid {
        #[cfg(target_os = "windows")]
        {
            let _ = tokio::process::Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = tokio::process::Command::new("pkill")
                .args(["-P", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
        }
    }

    let _ = child.kill().await;
    let _ = child.wait().await;
}
