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
    period: Option<&crate::runner::config::ExecutionPeriod>,
    timeout_seconds: u64,
) -> Result<()> {
    let resolved_executable = resolve_executable(&app.executable_path);
    let mut command = tokio::process::Command::new(&resolved_executable);

    if !app.config_path.trim().is_empty() {
        let resolved_config = resolve_relative_to_exe_dir(&app.config_path);
        command.arg("--config").arg(&resolved_config);
    }

    let mut effective_args = HashMap::new();
    let (start_str, end_str) = if let Some(p) = period {
        (
            p.start_date.format("%Y-%m-%d").to_string(),
            p.end_date.format("%Y-%m-%d").to_string(),
        )
    } else {
        (String::new(), String::new())
    };

    for (k, v) in args {
        let mut new_v = v.clone();
        if period.is_some() {
            new_v = new_v
                .replace("{start_date}", &start_str)
                .replace("{end_date}", &end_str);
        }
        effective_args.insert(k.clone(), new_v);
    }

    if period.is_some() {
        effective_args.insert("--start-date".to_string(), start_str.clone());
        effective_args.insert("--end-date".to_string(), end_str.clone());
    }

    let mut sorted_keys: Vec<&String> = effective_args.keys().collect();
    sorted_keys.sort_by(|a, b| {
        if a.as_str() == "-c" {
            std::cmp::Ordering::Less
        } else if b.as_str() == "-c" {
            std::cmp::Ordering::Greater
        } else {
            a.cmp(b)
        }
    });

    for k in sorted_keys {
        let v = &effective_args[k];
        if k == "--config" && !app.config_path.trim().is_empty() {
            // Do not allow task arguments to override the app's registered config path if the app already has one defined
            continue;
        }
        if v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("on") {
            command.arg(k);
        } else if v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("off") {
            // omit
        } else if v.trim().is_empty() {
            // Do not add the flag at all if its value is empty
        } else {
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
