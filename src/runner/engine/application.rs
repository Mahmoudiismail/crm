use anyhow::{Context, Result};
use std::collections::HashMap;

use crate::runner::config::RegisteredApp;
use crate::runner::engine::logging::TaskLogger;
use crate::runner::engine::process::{run_process, ProcessContext};
use crate::runner::engine::validation::{resolve_executable, resolve_relative_to_exe_dir};

const MANIFEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Reject an inverted ISO date range passed directly to an external app.
/// Other date formats are left to the target app because their semantics may differ.
fn validate_explicit_date_range(args: &HashMap<String, String>) -> Result<()> {
    let (Some(start), Some(end)) = (args.get("--start-date"), args.get("--end-date")) else {
        return Ok(());
    };
    let (Ok(start), Ok(end)) = (
        chrono::NaiveDate::parse_from_str(start.trim(), "%Y-%m-%d"),
        chrono::NaiveDate::parse_from_str(end.trim(), "%Y-%m-%d"),
    ) else {
        return Ok(());
    };
    if start > end {
        anyhow::bail!(
            "Invalid date range before launching external app: --start-date ({}) is after --end-date ({})",
            start,
            end
        );
    }
    Ok(())
}

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
        let Some(name) = arg.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let normalized = name
            .trim()
            .trim_start_matches('-')
            .to_ascii_lowercase()
            .replace('_', "-");
        let words: Vec<&str> = normalized.split('-').collect();
        let is_date_argument = arg.get("arg_type").and_then(serde_json::Value::as_str)
            == Some("date_var")
            || (words.contains(&"date")
                && (words
                    .iter()
                    .any(|word| matches!(*word, "start" | "from" | "end" | "to"))));
        if !is_date_argument {
            continue;
        }
        if start.is_none()
            && (words.iter().any(|word| *word == "start" || *word == "from"))
            && words.contains(&"date")
        {
            start = Some(name.to_string());
        } else if end.is_none()
            && (words.iter().any(|word| *word == "end" || *word == "to"))
            && words.contains(&"date")
        {
            end = Some(name.to_string());
        }
    }
    (start, end)
}

/// Check whether a manifest argument's declared dependencies are satisfied by the
/// arguments already configured for this invocation. Unknown arguments remain allowed
/// for backward compatibility with apps that do not expose a manifest.
fn manifest_arg_dependencies_satisfied(
    manifest: &serde_json::Value,
    arg_name: &str,
    effective_args: &HashMap<String, String>,
) -> bool {
    let Some(arg) = manifest
        .get("arguments")
        .and_then(serde_json::Value::as_array)
        .and_then(|args| {
            args.iter()
                .find(|arg| arg.get("name").and_then(serde_json::Value::as_str) == Some(arg_name))
        })
    else {
        return true;
    };

    let Some(dependencies) = arg.get("depends_on").and_then(serde_json::Value::as_object) else {
        return true;
    };

    dependencies
        .iter()
        .all(|(dependency_name, allowed_values)| {
            let Some(actual_value) = effective_args.get(dependency_name) else {
                return false;
            };
            allowed_values.as_array().is_some_and(|values| {
                values
                    .iter()
                    .any(|value| value.as_str() == Some(actual_value.as_str()))
            })
        })
}

fn insert_period_args(
    effective_args: &mut HashMap<String, String>,
    start_arg: Option<String>,
    end_arg: Option<String>,
    start_date: &str,
    end_date: &str,
) {
    if let Some(arg) = start_arg {
        effective_args.insert(arg, start_date.to_string());
    }
    if let Some(arg) = end_arg {
        effective_args.insert(arg, end_date.to_string());
    }
}

/// Returns whether this invocation's manifest exposes an applicable start/end date argument.
pub async fn supports_periodization(
    logger: &TaskLogger,
    app: &RegisteredApp,
    spec: &crate::runner::config::ExternalAppSpec,
) -> bool {
    if spec
        .args
        .values()
        .any(|value| value.contains("{start_date}") || value.contains("{end_date}"))
    {
        return true;
    }

    let (discovered_start, discovered_end, manifest) = discover_date_args(logger, app).await;
    let start = spec
        .start_date_arg
        .as_deref()
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .or(discovered_start)
        .filter(|name| {
            manifest
                .as_ref()
                .is_none_or(|value| manifest_arg_dependencies_satisfied(value, name, &spec.args))
        });
    let end = spec
        .end_date_arg
        .as_deref()
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .or(discovered_end)
        .filter(|name| {
            manifest
                .as_ref()
                .is_none_or(|value| manifest_arg_dependencies_satisfied(value, name, &spec.args))
        });
    start.is_some() || end.is_some()
}

async fn discover_date_args(
    logger: &TaskLogger,
    app: &RegisteredApp,
) -> (Option<String>, Option<String>, Option<serde_json::Value>) {
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
                Ok(manifest) => {
                    let (start, end) = date_args_from_manifest(&manifest);
                    (start, end, Some(manifest))
                }
                Err(error) => {
                    logger
                        .log(&format!(
                            "Could not parse manifest for app '{}': {}",
                            app.name, error
                        ))
                        .await;
                    (None, None, None)
                }
            }
        }
        Ok(Ok(output)) => {
            logger
                .log(&format!(
                    "Manifest discovery for app '{}' exited with status {}",
                    app.name, output.status
                ))
                .await;
            (None, None, None)
        }
        Ok(Err(error)) => {
            logger
                .log(&format!(
                    "Could not read manifest for app '{}': {}",
                    app.name, error
                ))
                .await;
            (None, None, None)
        }
        Err(_) => {
            logger
                .log(&format!(
                    "Manifest discovery timed out for app '{}'",
                    app.name
                ))
                .await;
            (None, None, None)
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
        // Explicit mappings take precedence, while manifest dependencies still
        // determine whether a configured argument is valid for this invocation.
        let discovered = discover_date_args(logger, app).await;
        let manifest = discovered.2.as_ref();
        let start_arg = spec
            .start_date_arg
            .as_deref()
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .or(discovered.0)
            .filter(|name| {
                manifest
                    .is_none_or(|m| manifest_arg_dependencies_satisfied(m, name, &effective_args))
            });
        let end_arg = spec
            .end_date_arg
            .as_deref()
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .or(discovered.1)
            .filter(|name| {
                manifest
                    .is_none_or(|m| manifest_arg_dependencies_satisfied(m, name, &effective_args))
            });

        let start_arg_name = start_arg.clone();
        let end_arg_name = end_arg.clone();
        insert_period_args(
            &mut effective_args,
            start_arg,
            end_arg,
            &start_str,
            &end_str,
        );
        logger
            .log(&format!(
                "Resolved date arguments for app '{}': start_arg={:?}, start_date={}, end_arg={:?}, end_date={}",
                app.name, start_arg_name, start_str, end_arg_name, end_str
            ))
            .await;
    }

    validate_explicit_date_range(&effective_args)?;

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

    // Do not write task command lines or arguments to runner.log.

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
    use super::{date_args_from_manifest, insert_period_args, manifest_arg_dependencies_satisfied};
    use std::collections::HashMap;

    #[test]
    fn respects_manifest_dependencies_for_explicit_date_arguments() {
        let manifest = serde_json::json!({
            "arguments": [
                {
                    "name": "--start-date",
                    "arg_type": "string",
                    "depends_on": {"--task": ["6"]}
                }
            ]
        });
        let wrong_task = HashMap::from([("--task".to_string(), "1".to_string())]);
        let sat_figures = HashMap::from([("--task".to_string(), "6".to_string())]);

        assert!(!manifest_arg_dependencies_satisfied(
            &manifest,
            "--start-date",
            &wrong_task
        ));
        assert!(manifest_arg_dependencies_satisfied(
            &manifest,
            "--start-date",
            &sat_figures
        ));
    }

    #[test]
    fn rejects_inverted_explicit_iso_date_args_before_launch() {
        let args = HashMap::from([
            ("--start-date".to_string(), "2026-10-18".to_string()),
            ("--end-date".to_string(), "2026-10-17".to_string()),
        ]);
        let error = super::validate_explicit_date_range(&args).unwrap_err();
        assert!(error.to_string().contains("--start-date (2026-10-18)"));
        assert!(error.to_string().contains("--end-date (2026-10-17)"));
    }

    #[test]
    fn accepts_valid_or_non_iso_explicit_date_args() {
        let valid = HashMap::from([
            ("--start-date".to_string(), "2026-10-17".to_string()),
            ("--end-date".to_string(), "2026-10-18".to_string()),
        ]);
        assert!(super::validate_explicit_date_range(&valid).is_ok());

        let non_iso = HashMap::from([
            ("--start-date".to_string(), "yesterday".to_string()),
            ("--end-date".to_string(), "today".to_string()),
        ]);
        assert!(super::validate_explicit_date_range(&non_iso).is_ok());
    }

    #[test]
    fn discovers_start_and_end_date_arguments_declared_as_strings() {
        let manifest = serde_json::json!({
            "arguments": [
                {"name": "--start-date", "arg_type": "string"},
                {"name": "--end-date", "arg_type": "string"}
            ]
        });
        assert_eq!(
            date_args_from_manifest(&manifest),
            (
                Some("--start-date".to_string()),
                Some("--end-date".to_string())
            )
        );
    }

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
    fn does_not_inject_date_flags_when_app_declares_none() {
        let mut args = HashMap::new();

        insert_period_args(&mut args, None, None, "2026-10-01", "2026-10-17");

        assert!(args.is_empty());
    }

    #[test]
    fn inserts_only_configured_or_discovered_date_flags() {
        let mut args = HashMap::new();

        insert_period_args(
            &mut args,
            Some("--from-date".to_string()),
            None,
            "2026-10-01",
            "2026-10-17",
        );

        assert_eq!(args.len(), 1);
        assert_eq!(
            args.get("--from-date").map(String::as_str),
            Some("2026-10-01")
        );
        assert!(!args.contains_key("--start-date"));
        assert!(!args.contains_key("--end-date"));
    }

    #[test]
    fn malformed_manifest_has_no_date_arguments() {
        assert_eq!(
            date_args_from_manifest(&serde_json::json!({})),
            (None, None)
        );
    }
}
