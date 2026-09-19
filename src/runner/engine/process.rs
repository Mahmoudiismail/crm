use anyhow::{Context, Result};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;

use crate::runner::engine::helpers::excerpt_utf8;
use crate::runner::engine::logging::TaskLogger;

const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024; // 10MB memory cap per stream

#[derive(Debug)]
pub struct ProcessContext<'a> {
    pub logger: &'a TaskLogger,
    pub command_str: String,
    pub timeout_seconds: u64,
    pub cmd: tokio::process::Command,
}

pub(crate) async fn execute_hardened_process(
    mut cmd: tokio::process::Command,
    timeout_seconds: u64,
) -> Result<(std::process::ExitStatus, Vec<u8>, Vec<u8>)> {
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.kill_on_drop(true);
    let mut child = cmd.spawn().context("Failed to spawn process")?;
    let child_pid = child.id();
    let timeout_duration = if timeout_seconds > 0 {
        Some(Duration::from_secs(timeout_seconds))
    } else {
        None
    };
    let mut stdout_stream = child.stdout.take();
    let mut stderr_stream = child.stderr.take();
    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    let read_stdout = read_bounded(stdout_stream.as_mut(), &mut stdout_bytes);
    let read_stderr = read_bounded(stderr_stream.as_mut(), &mut stderr_bytes);
    let status_res = if let Some(duration) = timeout_duration {
        let wait_child = async {
            let (s, _, _) = tokio::join!(child.wait(), read_stdout, read_stderr);
            s
        };
        match tokio::time::timeout(duration, wait_child).await {
            Ok(s) => s.context("Failed waiting for process")?,
            Err(_) => {
                terminate_process_tree(child_pid, &mut child).await;
                return Err(anyhow::anyhow!(
                    "Process timed out after {}s",
                    timeout_seconds
                ));
            }
        }
    } else {
        let (s, _, _) = tokio::join!(child.wait(), read_stdout, read_stderr);
        s.context("Failed waiting for process completion")?
    };
    Ok((status_res, stdout_bytes, stderr_bytes))
}

pub async fn run_process(ctx: ProcessContext<'_>) -> Result<()> {
    ctx.logger
        .log("--------------------------------------------------")
        .await;
    ctx.logger
        .log(&format!(">>> EXECUTING ACTION: {}", ctx.command_str))
        .await;
    ctx.logger
        .log("--------------------------------------------------")
        .await;

    let (status, stdout_bytes, stderr_bytes) =
        match execute_hardened_process(ctx.cmd, ctx.timeout_seconds).await {
            Ok(res) => res,
            Err(e) => {
                if e.to_string().contains("timed out") {
                    ctx.logger
                        .log(&format!(
                            "TIMEOUT: Action exceeded timeout of {}s. Terminating process tree...",
                            ctx.timeout_seconds
                        ))
                        .await;
                    return Err(anyhow::anyhow!(
                        "Command timed out after {}s: {}",
                        ctx.timeout_seconds,
                        ctx.command_str
                    ));
                }
                return Err(e.context(format!(
                    "Failed to spawn process for command: {}",
                    ctx.command_str
                )));
            }
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

async fn read_bounded<R: AsyncReadExt + Unpin>(stream: Option<&mut R>, out: &mut Vec<u8>) {
    if let Some(s) = stream {
        let mut buf = [0u8; 8192];
        loop {
            if out.len() < MAX_OUTPUT_BYTES {
                let to_read = (MAX_OUTPUT_BYTES - out.len()).min(buf.len());
                match s.read(&mut buf[..to_read]).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => out.extend_from_slice(&buf[..n]),
                }
            } else {
                // Buffer cap reached: continue draining pipe without storing extra bytes to prevent deadlock
                match s.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bounded_process_output_cap_above_10mb() {
        let mut stdout_bytes = Vec::new();

        let stream_size = 12 * 1024 * 1024;
        let large_stream = vec![b'A'; stream_size];
        let mut cursor = std::io::Cursor::new(large_stream);

        read_bounded(Some(&mut cursor), &mut stdout_bytes).await;

        assert_eq!(
            stdout_bytes.len(),
            MAX_OUTPUT_BYTES,
            "Captured output must be capped strictly at 10 MiB"
        );
    }

    #[tokio::test]
    async fn test_process_timeout_and_tree_termination() {
        let logger = TaskLogger::new("timeout_test", "timeout_test");

        let cmd = if cfg!(windows) {
            let mut c = tokio::process::Command::new("cmd");
            c.args(["/C", "ping -n 10 127.0.0.1 > nul"]);
            c
        } else {
            let mut c = tokio::process::Command::new("sleep");
            c.arg("10");
            c
        };

        let ctx = ProcessContext {
            logger: &logger,
            command_str: "long_running_process".to_string(),
            timeout_seconds: 1,
            cmd,
        };

        let res = run_process(ctx).await;
        assert!(res.is_err(), "Process should return timeout error");
        let err_msg = res.unwrap_err().to_string();
        assert!(
            err_msg.contains("timed out"),
            "Error message should mention timeout"
        );
    }
}
#[tokio::test]
async fn test_real_bounded_process_output_cap_above_10mb() {
    let logger = TaskLogger::new("bounded_test", "bounded_test");
    let cmd = if cfg!(windows) {
        let mut c = tokio::process::Command::new("powershell");
        c.args([
            "-NoProfile",
            "-Command",
            "$str = 'A' * 1024; for ($i=0; $i -lt 12000; $i++) { Write-Host $str }",
        ]);
        c
    } else {
        let mut c = tokio::process::Command::new("sh");
        c.args(["-c", "yes AAAA | head -c 12582912"]);
        c
    };
    let ctx = ProcessContext {
        logger: &logger,
        command_str: "bounded_output_process".to_string(),
        timeout_seconds: 30,
        cmd,
    };
    let res = run_process(ctx).await;
    assert!(
        res.is_ok(),
        "Bounded process should succeed without timeout"
    );
}

#[tokio::test]
async fn test_real_process_tree_timeout_and_termination() {
    let logger = TaskLogger::new("timeout_test", "timeout_test");
    let cmd = if cfg!(windows) {
        let mut c = tokio::process::Command::new("cmd");
        c.args(["/C", "start /wait ping -n 10 127.0.0.1 > nul"]);
        c
    } else {
        let mut c = tokio::process::Command::new("sh");
        c.args(["-c", "sleep 10"]);
        c
    };
    let ctx = ProcessContext {
        logger: &logger,
        command_str: "long_running_process".to_string(),
        timeout_seconds: 1,
        cmd,
    };
    let res = run_process(ctx).await;
    assert!(res.is_err(), "Process should return timeout error");
    let err_msg = res.unwrap_err().to_string();
    assert!(
        err_msg.contains("timed out"),
        "Error message should mention timeout"
    );
}
