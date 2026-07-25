use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::trace;

use crate::runner::config::RegisteredApp;
use crate::runner::engine::logging::TaskLogger;
use crate::runner::engine::process::{run_process, ProcessContext};
use crate::runner::engine::validation::{resolve_executable, resolve_relative_to_exe_dir};

pub async fn run_external_app(
    logger: &TaskLogger,
    app: &RegisteredApp,
    args: &HashMap<String, String>,
    timeout_seconds: u64,
) -> Result<()> {
    let resolved_executable = resolve_executable(&app.executable_path);
    let mut command = tokio::process::Command::new(&resolved_executable);

    if !app.config_path.trim().is_empty() {
        let resolved_config = resolve_relative_to_exe_dir(&app.config_path);
        command.arg("--config").arg(&resolved_config);
    }

    for (k, v) in args {
        if k == "--config" && !app.config_path.trim().is_empty() {
            // Do not allow task arguments to override the app's registered config path if the app already has one defined
            continue;
        }
        if v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("on") {
            command.arg(k);
        } else if v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("off") {
            // omit
        } else if v.trim().is_empty() {
            // Do not add the flag at all if its value is empty, this prevents passing empty filters like `--filters ""` or empty `--config ""`
        } else {
            // The UI form_script.js joins arrays with comma for MultiList so this will correctly pass `--arg a,b,c`
            // Clap handles this when `value_delimiter = ','` is set on the arg.
            command.arg(k).arg(v);
        }
    }

    trace!("Command to execute: {:?}", command);

    let ctx = ProcessContext {
        logger,
        command_str: format!(
            "external app '{}' ({})",
            app.name,
            resolved_executable.display()
        ),
        timeout_seconds,
        cmd: command,
    };

    run_process(ctx).await.with_context(|| {
        format!(
            "external app '{}' failed ({})",
            app.name,
            resolved_executable.display(),
        )
    })
}
