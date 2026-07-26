use anyhow::{Context, Result};
use std::process::Stdio;
use std::time::Duration;

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

    let output = if ctx.timeout_seconds == 0 {
        ctx.cmd.output().await?
    } else {
        tokio::time::timeout(Duration::from_secs(ctx.timeout_seconds), ctx.cmd.output())
            .await
            .with_context(|| {
                format!(
                    "Command timed out after {}s: {}",
                    ctx.timeout_seconds, ctx.command_str
                )
            })??
    };

    ctx.logger.log_bytes("STDOUT", &output.stdout).await;
    ctx.logger.log_bytes("STDERR", &output.stderr).await;

    if !output.status.success() {
        let stdout_excerpt = excerpt_utf8(&output.stdout);
        let stderr_excerpt = excerpt_utf8(&output.stderr);
        return Err(anyhow::anyhow!(
            "Command failed with exit code {:?}\n\nSTDOUT EXCERPT:\n{}\n\nSTDERR EXCERPT:\n{}",
            output.status.code(),
            stdout_excerpt,
            stderr_excerpt
        ));
    }

    Ok(())
}
