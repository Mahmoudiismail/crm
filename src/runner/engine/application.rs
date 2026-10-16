use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::{trace, warn};

use crate::runner::config::RegisteredApp;
use crate::runner::engine::logging::TaskLogger;
use crate::runner::engine::process::{run_process, ProcessContext};
use crate::runner::engine::validation::{resolve_executable, resolve_relative_to_exe_dir};

const MANIFEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Discover conventional start/end date argument names from an app's manifest.
/// Only date-like arguments with an unambiguous start/from or end/to name are used.
fn date_args_from_manifest(manifest: &serde_json::Value) -> (Option<String>, Option<String>) {
    let Some(arguments) = manifest
        .get("arguments")
        .and_then(serde_json::Value::as_array)
    else {
        return (None, None);
    };

    let mut start = None;
    let mut end = None;
    for arg in arguments {
        if arg.get("arg_type").and_then(serde_json::Value::as_str) != Some("date_var") {
            continue;
        }
        let Some(name) = arg.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let normalized = name
            .trim()
            .trim_start_matches('-')
            .to_ascii_lowercase()
            .replace('_', "-");
        let words: Vec<&str> = normalized.split('-').collect();
        if start.is_none()
            && (words.iter().any(|word| *word == "start" || *word == "from"))
            && words.iter().any(|word| *word == "date")
        {
            start = Some(name.to_string());
        } else if end.is_none()
            && (words.iter().any(|word| *word == "end" || *word == "to"))
            && words.iter().any(|word| *word == "date")
        {
            end = Some(name.to_string());
        }
    }
    (start, end)
}

async fn discover_date_args(app: &RegisteredApp) -> (Option<String>, Option<String>) {
    let executable = resolve_executable(&app.executable_path);
    let mut command = tokio::process::Command::new(executable);
    command
        .arg("--manifest")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);

    match tokio::time::timeout(MANIFEST_TIMEOUT, command.output()).await {
        Ok(Ok(output)) if output.status.success() => {
            match serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                Ok(manifest) => date_args_from_manifest(&manifest),
                Err(error) => {
                    warn!("Could not parse manifest for app '{}': {}", app.name, error);
                    (None, None)
                }
            }
        }
        Ok(Ok(output)) => {
            warn!(
                "Manifest discovery for app '{}' exited with status {}",
                app.name, output.status
            );
            (None, None)
        }
        Ok(Err(error)) => {
            warn!("Could not read manifest for app '{}': {}", app.name, error);
            (None, None)
        }
        Err(_) => {
            warn!("Manifest discovery timed out for app '{}'", app.name);
            (None, None)
        }
    }
}

pub async fn run_external_app(
    logger: &TaskLogger,
    app: &RegisteredApp,
    spec: &crate::runner::config::ExternalAppSpec,
    period: Option<&crate::runner::config::ExecutionPeriod>,
    timeout_seconds: u64,
) -> Result<()> {
    let resolved_executable = resolve_executable(&app.executable_path);
    let mut command = tokio::process::Command::new(&resolved_executable);

    if !app.config_path.trim().is_empty() {
        let resolved_config = resolve_relative_to_exe_dir(&app.config_path);
        command.arg("--config").arg(&resolved_config);
    }

    let args = &spec.args;
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
        // Explicit task mappings always win. If they are absent, use the app manifest
        // to discover non-standard date flags, then retain the legacy flag names as fallback.
        let needs_manifest = spec.start_date_arg.as_deref().map_or(true, str::is_empty)
            || spec.end_date_arg.as_deref().map_or(true, str::is_empty);
        let discovered = if needs_manifest {
            discover_date_args(app).await
        } else {
            (None, None)
        };
        let start_arg = spec
            .start_date_arg
            .as_deref()
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .or(discovered.0)
            .unwrap_or_else(|| "--start-date".to_string());
        let end_arg = spec
            .end_date_arg
            .as_deref()
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .or(discovered.1)
            .unwrap_or_else(|| "--end-date".to_string());

        effective_args.insert(start_arg, start_str.clone());
        effective_args.insert(end_arg, end_str.clone());
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

#[cfg(test)]
mod tests {
    use super::date_args_from_manifest;

    #[test]
    fn discovers_standard_and_custom_date_var_names() {
        let manifest = serde_json::json!({
            "arguments": [
                {"name": "--report", "arg_type": "string"},
                {"name": "--from-date", "arg_type": "date_var"},
                {"name": "--to-date", "arg_type": "date_var"}
            ]
        });
        assert_eq!(
            date_args_from_manifest(&manifest),
            (
                Some("--from-date".to_string()),
                Some("--to-date".to_string())
            )
        );
    }

    #[test]
    fn ignores_ambiguous_and_non_date_arguments() {
        let manifest = serde_json::json!({
            "arguments": [
                {"name": "--date", "arg_type": "date_var"},
                {"name": "--start-date", "arg_type": "string"},
                {"name": "--report", "arg_type": "string"}
            ]
        });
        assert_eq!(date_args_from_manifest(&manifest), (None, None));
    }

    #[test]
    fn malformed_manifest_has_no_date_arguments() {
        assert_eq!(
            date_args_from_manifest(&serde_json::json!({})),
            (None, None)
        );
    }
}
