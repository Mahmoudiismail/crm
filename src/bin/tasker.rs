use anyhow::{Context, Result};
use clap::Parser;
use crm_tool::manifest::{AppArg, AppManifest, ArgType};
use crm_tool::tasker::config::{TaskConfig, TaskerConfig};
use crm_tool::tasker::{csv_task, dashboard_updater};
use crm_tool::utils::{executable_dir, intercept_manifest, setup_logging};
use serde_json::Value;

use std::fs;
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "tasker", about = "Tasker Reporting Tool")]
pub struct TaskerCliOptions {
    #[arg(long)]
    pub config: Option<String>,
    #[arg(long)]
    pub task: Option<usize>,
    #[arg(long)]
    pub only_call_center: bool,
    #[arg(long)]
    pub send_exceptions: bool,
    #[arg(long, hide = true)]
    pub manifest: bool,

    // Support the legacy positional argument for config
    #[arg(hide = true)]
    pub legacy_config: Option<String>,
}

pub fn merge_configs(a: &mut Value, b: &Value, changed: &mut bool) {
    match (a, b) {
        (Value::Object(a_obj), Value::Object(b_obj)) => {
            for (k, v) in b_obj {
                if let Some(a_val) = a_obj.get_mut(k) {
                    merge_configs(a_val, v, changed);
                } else {
                    a_obj.insert(k.clone(), v.clone());
                    *changed = true;
                }
            }
        }
        (Value::Array(a_arr), Value::Array(b_arr)) => {
            let len = std::cmp::min(a_arr.len(), b_arr.len());
            for i in 0..len {
                merge_configs(&mut a_arr[i], &b_arr[i], changed);
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

pub fn run_app(options: TaskerCliOptions) -> Result<()> {
    info!("Tasker started.");

    let config_path_arg = options.config.or(options.legacy_config).map(PathBuf::from);
    let task_filter = options.task;
    let only_call_center = options.only_call_center;
    let send_exceptions = options.send_exceptions;

    let default_config_path = executable_dir().join("tasker_config.json");
    let config_path = config_path_arg.unwrap_or(default_config_path);

    let default_config_content = r#"{

  "tasks": [
    {
      "type": "csv_analysis",
      "download_path": "../crm_windows/Downloads",
      "users_file": "./task1/users.csv",
      "assignment_settings_file": "./task1/assignments.csv",
      "minutes_ago": 15,
      "start_date": "01-May-2026",
      "exclude_branches": [
        "Dr. Soliman Fakeeh Hospital Madinah",
        "Medical Fakeeh"
      ],
      "exclude_categories": [
        "incomplete reservation"
      ],
      "category_exceptions": [],
      "output_file": "./results.csv",
      "email_config": {
        "team_mapping_file": "./teams.csv",
        "body_template_file": "./email_template.html",
        "initial_cc": "initial@example.com",
        "ending_cc": "ending@example.com",
        "send_emails": false,
        "default_to_email": "fallback@example.com",
        "send_per_team_all_branches": [],
        "send_per_team_branches": [
          "Dr. Soliman Fakeeh Hospital"
        ],
        "send_per_branch_branches": [
          "dsfmc",
          "DSFMH"
        ],
        "send_call_center": false,
        "send_exceptions": false,
        "indentation_spaces": 4
      }
    },
    {
      "type": "dashboard_updater",
      "download_path": "../crm_windows/Downloads",
      "users_file": "./task2/users.csv",
      "assignment_settings_file": "./task2/assignments.csv",
      "minutes_ago": 15,
      "exclude_branches": [],
      "exclude_categories": [],
      "output_file": "./dashboard_results.csv",
      "dashboard_file": "./dashboard.xlsx",
      "email_to": "dash@example.com",
      "email_cc": "cc@example.com",
      "save_email_as_html": false,
      "indentation_spaces": 4
    },
    {
      "type": "crm_open_sohail",
      "download_path": "../crm_windows/Downloads",
      "users_file": "./task3/users.csv",
      "assignment_settings_file": "./task3/assignments.csv",
      "minutes_ago": 15,
      "start_date": null,
      "exclude_branches": [],
      "exclude_categories": [],
      "category_exceptions": null,
      "output_file": "./crm_open_sohail_results.csv",
      "dashboard_file": "./dashboard_sohail.xlsx",
      "email_to": "",
      "email_cc": "",
      "save_email_as_html": false,
      "indentation_spaces": 4,
      "team_mapping_file": "./task3/teams.csv",
      "body_template_file": null,
      "subject_template": null,
      "branch_filter": null,
      "month_filter": null,
      "fallback_oul": "N/A",
      "dashboard_sheet_name": "Sheet1",
      "dashboard_pivot_name": "PivotTable2",
      "table_column_widths": ["15%", "10%", "10%", "15%", "15%", "15%", "20%"]
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

    let mut config_changed = false;
    merge_configs(
        &mut current_config_val,
        &default_config_val,
        &mut config_changed,
    );

    let config: TaskerConfig = serde_json::from_value(current_config_val.clone())
        .with_context(|| "Failed to parse tasker_config.json into TaskerConfig")?;

    if let Some(filter) = task_filter {
        if filter == 0 || filter > config.tasks.len() {
            anyhow::bail!("Task filter index {} is out of bounds. The configuration only contains {} task(s).", filter, config.tasks.len());
        }
    }

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

    for (i, task) in config.tasks.iter().enumerate() {
        let task_idx = i + 1;

        tracing::trace!("Processing task #{} from configuration.", task_idx);
        if let Some(filter) = task_filter {
            if task_idx != filter {
                tracing::trace!(
                    "Skipping task #{} due to filter (target: {}).",
                    task_idx,
                    filter
                );
                continue;
            }
        }

        info!("Running task #{}", task_idx);
        match task {
            TaskConfig::CsvAnalysis(csv_config) => {
                tracing::trace!("Executing CsvAnalysis for task #{}.", task_idx);
                if let Err(e) = csv_task::run(csv_config, only_call_center, send_exceptions) {
                    error!("Error running CsvAnalysis task #{}: {:?}", task_idx, e);
                }
                tracing::trace!("CsvAnalysis for task #{} finished.", task_idx);
            }
            TaskConfig::DashboardUpdater(dash_config) => {
                tracing::trace!("Executing DashboardUpdater for task #{}.", task_idx);
                if let Err(e) = dashboard_updater::run(dash_config) {
                    error!("Error running DashboardUpdater task #{}: {:?}", task_idx, e);
                }
                tracing::trace!("DashboardUpdater for task #{} finished.", task_idx);
            }
            TaskConfig::CrmOpenSohail(sohail_config) => {
                tracing::trace!("Executing CrmOpenSohail for task #{}.", task_idx);
                if let Err(e) = crm_tool::tasker::crm_open_sohail::run(sohail_config) {
                    error!("Error running CrmOpenSohail task #{}: {:?}", task_idx, e);
                    anyhow::bail!("CrmOpenSohail task {} failed: {}", task_idx, e);
                }
                tracing::trace!("CrmOpenSohail for task #{} finished.", task_idx);
            }
        }
    }

    info!("All tasks completed.");
    Ok(())
}

fn get_manifest() -> AppManifest {
    AppManifest {
        name: "Tasker Reporting Tool".to_string(),
        description:
            "Executes configured background workflows such as CSV analysis and email dispatching."
                .to_string(),
        arguments: vec![
            AppArg {
                name: "--config".to_string(),
                arg_type: ArgType::String,
                required: false,
                default_value: None,
                options: None,
                depends_on: None,
                autofill: None,
            },
            AppArg {
                name: "--task".to_string(),
                arg_type: ArgType::Number,
                required: false,
                default_value: None,
                options: None,
                depends_on: None,
                autofill: None,
            },
            AppArg {
                name: "--only-call-center".to_string(),
                arg_type: ArgType::Boolean,
                required: false,
                default_value: Some("false".to_string()),
                options: None,
                depends_on: None,
                autofill: None,
            },
            AppArg {
                name: "--send-exceptions".to_string(),
                arg_type: ArgType::Boolean,
                required: false,
                default_value: Some("false".to_string()),
                options: None,
                depends_on: None,
                autofill: None,
            },
        ],
    }
}

fn main() -> Result<()> {
    intercept_manifest(get_manifest());

    let _guard = setup_logging("task_csv_analysis")?;

    let options = TaskerCliOptions::parse();

    if let Err(e) = run_app(options) {
        error!("Fatal application error: {:#}", e);
        anyhow::bail!(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_tasker_args_parsing() {
        let tmp = std::env::temp_dir();
        let config_path = tmp.join("mock_tasker_config.json");
        let _ = std::fs::remove_file(&config_path);

        let args = vec![
            "tasker".to_string(),
            "--config".to_string(),
            config_path.to_string_lossy().to_string(),
            "--only-call-center".to_string(),
            "--send-exceptions".to_string(),
        ];

        let options = TaskerCliOptions::parse_from(args);
        let _res = run_app(options);
        let _ = std::fs::remove_file(&config_path);
    }

    #[test]
    fn test_task_filtering_logic_valid_index() {
        // We test run_app's integration directly using a mock file on disk
        use std::io::Write;

        let tmp = std::env::temp_dir();
        let config_path = tmp.join("mock_tasker_config_valid.json");

        let mock_config_json = serde_json::json!({
            "tasks": [
                {
                    "type": "csv_analysis",
                    "download_path": "path1",
                    "users_file": "u1",
                    "assignment_settings_file": "a1",
                    "minutes_ago": 10,
                    "exclude_branches": [],
                    "exclude_categories": [],
                    "output_file": "out1"
                },
                {
                    "type": "csv_analysis",
                    "download_path": "path2",
                    "users_file": "u2",
                    "assignment_settings_file": "a2",
                    "minutes_ago": 20,
                    "exclude_branches": [],
                    "exclude_categories": [],
                    "output_file": "out2"
                }
            ]
        });

        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(mock_config_json.to_string().as_bytes())
            .unwrap();
        file.sync_all().unwrap();

        // Passing valid task index 2. This won't actually succeed all the way because 'path2' doesn't exist,
        // but it WILL pass the bounds check and fail further down the execution tree.
        let args = vec![
            "tasker".to_string(),
            "--config".to_string(),
            config_path.to_string_lossy().to_string(),
            "--task".to_string(),
            "2".to_string(),
        ];

        let options = TaskerCliOptions::parse_from(args);

        // We know it won't bail on BoundsCheck.
        // It will return Ok(()) but inside `csv_task::run` it logs an error if path2 doesn't exist.
        // run_app itself returns Ok(()) if internal task errors are caught and logged.
        let res = run_app(options);
        assert!(
            res.is_ok(),
            "run_app should successfully route the valid task filter"
        );

        let _ = std::fs::remove_file(&config_path);
    }

    #[test]
    fn test_task_filtering_logic_out_of_bounds() {
        use std::io::Write;

        let tmp = std::env::temp_dir();
        let config_path = tmp.join("mock_tasker_config_oob.json");

        let mock_config_json = serde_json::json!({
            "tasks": [
                {
                    "type": "csv_analysis",
                    "download_path": "path1",
                    "users_file": "u1",
                    "assignment_settings_file": "a1",
                    "minutes_ago": 10,
                    "output_file": "out1"
                }
            ]
        });

        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(mock_config_json.to_string().as_bytes())
            .unwrap();
        file.sync_all().unwrap();

        // Pass task 5, which does not exist.
        let args = vec![
            "tasker".to_string(),
            "--config".to_string(),
            config_path.to_string_lossy().to_string(),
            "--task".to_string(),
            "5".to_string(),
        ];

        let options = TaskerCliOptions::parse_from(args);
        let res = run_app(options);
        assert!(
            res.is_err(),
            "run_app MUST bail when the task index is out of bounds"
        );
        assert!(
            res.unwrap_err().to_string().contains("out of bounds"),
            "Error message should mention bounds"
        );

        let _ = std::fs::remove_file(&config_path);
    }

    #[test]
    fn test_merge_function() {
        use serde_json::json;

        let default_config = json!({
            "tasks": [
                {
                    "type": "csv_analysis",
                    "minutes_ago": 15,
                    "email_config": {
                        "send_emails": false,
                        "default_to_email": "fallback@example.com"
                    }
                }
            ]
        });

        let mut user_config = json!({
            "tasks": [
                {
                    "type": "csv_analysis",
                    "email_config": {
                        "send_emails": true
                    }
                }
            ]
        });

        let mut changed = false;
        merge_configs(&mut user_config, &default_config, &mut changed);

        assert!(changed, "Merge should mark config as changed");

        let merged_task = &user_config["tasks"][0];
        assert_eq!(
            merged_task["minutes_ago"], 15,
            "Should merge root level fields"
        );
        assert_eq!(
            merged_task["email_config"]["send_emails"], true,
            "Should NOT overwrite existing user fields"
        );
        assert_eq!(
            merged_task["email_config"]["default_to_email"], "fallback@example.com",
            "Should merge nested missing fields"
        );
    }
}
