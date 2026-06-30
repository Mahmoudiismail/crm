use anyhow::{Context, Result};
use crm_tool::tasker::config::{TaskConfig, TaskerConfig};
use crm_tool::tasker::csv_task;
use serde_json::Value;
use std::env;
use std::fs;
use tracing::{error, info};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn run_app(args: Vec<String>) -> Result<()> {
    info!("Tasker started.");

    // Parse arguments

    let mut config_path_arg = None;
    let mut task_filter: Option<usize> = None;
    let mut only_call_center = false;
    let mut send_exceptions = false;

    let skip_count = if args.first().is_some_and(|a| !a.starts_with('-')) {
        1
    } else {
        0
    };
    let mut args_iter = args.into_iter().skip(skip_count).peekable();
    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "--config" => {
                if let Some(path) = args_iter.peek() {
                    if !path.starts_with("-") {
                        config_path_arg = Some(std::path::PathBuf::from(args_iter.next().unwrap()));
                    } else {
                        tracing::warn!(
                            "--config flag provided but next argument starts with '-'. Ignoring."
                        );
                    }
                }
            }
            "--task" => {
                if let Some(task_num) = args_iter.peek() {
                    if let Ok(idx) = task_num.parse::<usize>() {
                        task_filter = Some(idx);
                        args_iter.next(); // Consume the number
                    }
                }
            }
            "--only-call-center" => {
                only_call_center = true;
            }
            "--send-exceptions" => {
                send_exceptions = true;
            }
            // Support the legacy positional config path if they don't provide a flag
            val if !val.starts_with("-") && config_path_arg.is_none() => {
                config_path_arg = Some(std::path::PathBuf::from(val));
            }
            _ => {}
        }
    }

    let default_config_path = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("tasker_config.json")))
        .unwrap_or_else(|| std::path::PathBuf::from("tasker_config.json"));

    let config_path = config_path_arg.unwrap_or(default_config_path);

    let default_config_content = r#"{

  "tasks": [
    {
      "type": "csv_analysis",
      "download_path": "../crm_windows/Downloads",
      "users_file": "./task1/users.csv",
      "assignment_settings_file": "./task1/assignments.csv",
      "minutes_ago": 15,
      "exclude_branches": [
        "Dr. Soliman Fakeeh Hospital Madinah",
        "Medical Fakeeh"
      ],
      "exclude_categories": [
        "incomplete reservation"
      ],
      "category_exceptions": [
        {
          "category": "incomplete reservation",
          "branch": "",
          "team": ""
        }
      ],
      "output_file": "./results.csv",
      "email_config": {
        "team_mapping_file": "./teams.csv",
        "body_template_file": "./email_template.html",
        "initial_cc": "initial@example.com",
        "ending_cc": "ending@example.com",
        "send_emails": false,
        "default_to_email": "fallback@example.com",
        "send_per_team_branches": [
          "Dr. Soliman Fakeeh Hospital"
        ],
        "send_per_branch_branches": [
          "dsfmc",
          "DSFMH"
        ],
        "send_call_center": false
      }
    }
  ]
}"#;

    if !config_path.exists() {
        info!(
            "Configuration file not found at {}. Generating default configuration.",
            config_path.display()
        );

        fs::write(&config_path, default_config_content).with_context(|| {
            format!(
                "Failed to write default tasker config at {}",
                config_path.display()
            )
        })?;
    }

    info!("Loading configuration from: {}", config_path.display());
    let config_content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read tasker config at {}", config_path.display()))?;

    let mut current_config_val: Value = serde_json::from_str(&config_content)
        .with_context(|| "Failed to parse tasker_config.json as JSON")?;

    let default_config_val: Value = serde_json::from_str(default_config_content)
        .with_context(|| "Failed to parse default config as JSON")?;

    fn merge(a: &mut Value, b: &Value, changed: &mut bool) {
        match (a, b) {
            (Value::Object(a_obj), Value::Object(b_obj)) => {
                for (k, v) in b_obj {
                    if !a_obj.contains_key(k) {
                        a_obj.insert(k.clone(), v.clone());
                        *changed = true;
                    } else {
                        merge(a_obj.get_mut(k).unwrap(), v, changed);
                    }
                }
            }
            (Value::Array(a_arr), Value::Array(b_arr)) => {
                // If it's the tasks array, we might want to merge into each task
                // For simplicity, we just merge into existing elements up to the min length
                // If the user's config has fewer tasks than default, we don't necessarily append defaults,
                // but if we did, we'd add it. We'll leave array merging simple.
                let len = std::cmp::min(a_arr.len(), b_arr.len());
                for i in 0..len {
                    merge(&mut a_arr[i], &b_arr[i], changed);
                }
            }
            (a_val, b_val) => {
                if a_val.is_null() && !b_val.is_null() {
                    *a_val = b_val.clone();
                    *changed = true;
                }
            }
        }
    }

    let mut config_changed = false;
    merge(
        &mut current_config_val,
        &default_config_val,
        &mut config_changed,
    );

    if config_changed {
        info!("Updated configuration file with missing default fields.");
        let updated_content = serde_json::to_string_pretty(&current_config_val)
            .with_context(|| "Failed to serialize updated config")?;
        fs::write(&config_path, updated_content).with_context(|| {
            format!(
                "Failed to write updated tasker config at {}",
                config_path.display()
            )
        })?;
    }

    let config: TaskerConfig = serde_json::from_value(current_config_val)
        .with_context(|| "Failed to parse tasker_config.json into TaskerConfig")?;

    for (i, task) in config.tasks.iter().enumerate() {
        let task_idx = i + 1;

        if let Some(filter) = task_filter {
            if task_idx != filter {
                continue;
            }
        }

        info!("Running task #{}", task_idx);
        match task {
            TaskConfig::CsvAnalysis(csv_config) => {
                if let Err(e) = csv_task::run(csv_config, only_call_center, send_exceptions) {
                    error!("Error running CsvAnalysis task: {:?}", e);
                }
            }
        }
    }

    info!("All tasks completed.");
    Ok(())
}

fn main() -> Result<()> {
    // Setup file logging in the same directory as the executable
    let log_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let file_appender = RollingFileAppender::new(Rotation::NEVER, log_dir, "task_csv_analysis.log");

    // We can also have a stdout logger if desired, but we'll stick to a simple combined setup.
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(file_appender))
        .with(fmt::layer().with_writer(std::io::stdout))
        .init();

    if let Err(e) = run_app(env::args().collect()) {
        error!("Fatal application error: {:#}", e);
        anyhow::bail!(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tasker_args_parsing() {
        // Run with mock args and a fake config path
        let tmp = std::env::temp_dir();
        let config_path = tmp.join("mock_tasker_config.json");
        let _ = std::fs::remove_file(&config_path);

        let args = vec![
            "tasker".to_string(),
            "--config".to_string(),
            config_path.to_str().unwrap().to_string(),
            "--only-call-center".to_string(),
            "--send-exceptions".to_string(),
        ];

        let _res = run_app(args);
        // It should succeed or fail cleanly, but mostly we just verify it doesn't crash on parse
        // It will actually create the mock config and then exit cleanly or with an error
        let _ = std::fs::remove_file(&config_path);
    }
}
