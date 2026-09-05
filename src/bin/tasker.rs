use anyhow::{Context, Result};
use clap::Parser;
use crm_tool::manifest::{AppArg, AppManifest, ArgType};
use crm_tool::tasker::config::{TaskConfig, TaskerConfig};
use crm_tool::tasker::{csv_task, dashboard_updater, opd_task};
use crm_tool::utils::{
    executable_dir, intercept_manifest, parse_log_level, setup_logging_with_levels, InterceptResult,
};

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

pub fn run_app(options: TaskerCliOptions) -> Result<()> {
    info!("Tasker started.");

    let config_path_arg = options.config.or(options.legacy_config).map(PathBuf::from);
    let task_filter = options.task;
    let only_call_center = options.only_call_center;
    let send_exceptions = options.send_exceptions;

    let default_config_path = executable_dir()?.join("tasker_config.json");
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
      "output_file": "./results.csv",
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
      "fallback_oul": "",
      "dashboard_sheet_name": "Sheet1",
      "dashboard_pivot_name": "PivotTable2",
      "table_column_widths": ["15%", "10%", "10%", "15%", "15%", "15%", "20%"],
      "sender_account_email": "default@example.com",
      "reply_subject_prefix": "[CRM]"
    },
    {
      "type": "department_split",
      "dashboard_file": "./dashboard.xlsx",
      "chair_file": "./task4/chair.csv",
      "output_dir": "./split_reports"
    },
    {
      "type": "opd_analysis",
      "download_path": "../crm_windows/Downloads",
      "cus_input": "./task5/cus_input.csv",
      "cus_file": "./cus_output.csv",
      "exclude_specialities": ["ECG", "Laser Hair Removal"],
      "exclude_emp_names": ["Echo Doctor 2", "Neurophysiology", "Obgyn Imaging Routine", "Pre Marital Screening Doctor"],
      "exclude_depts": ["Khadija Attar Center for Special Needs", "Patient Education"],
      "exclude_speciality_prefixes": ["Exe"],
      "email_to": "dd@merrywillow.mailk.us",
      "email_subject": "201009977888-1576573266@g.us",
      "special_column_name": "Special",
      "date_column_name": "KSA Time",
      "check_current_year": true
    }
  ]
}"#;

    let default_config: TaskerConfig =
        serde_json::from_str(default_config_content).with_context(|| {
            format!(
                "Failed to parse default config string as JSON. content: {}",
                default_config_content
            )
        })?;

    let config = crm_tool::utils::load_or_create_config(&config_path, &default_config)?;

    config.validate()?;
    info!("Loaded config: {:#?}", config);

    if let Some(filter) = task_filter {
        if filter == 0 || filter > config.tasks.len() {
            anyhow::bail!("Task filter index {} is out of bounds. The configuration only contains {} task(s).", filter, config.tasks.len());
        }
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
                    anyhow::bail!("CsvAnalysis task {} failed: {}", task_idx, e);
                }
                tracing::trace!("CsvAnalysis for task #{} finished.", task_idx);
            }
            TaskConfig::DashboardUpdater(dash_config) => {
                tracing::trace!("Executing DashboardUpdater for task #{}.", task_idx);
                if let Err(e) = dashboard_updater::run(dash_config) {
                    error!("Error running DashboardUpdater task #{}: {:?}", task_idx, e);
                    anyhow::bail!("DashboardUpdater task {} failed: {}", task_idx, e);
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
            TaskConfig::DepartmentSplit(split_config) => {
                tracing::trace!("Executing DepartmentSplit for task #{}.", task_idx);
                if let Err(e) = crm_tool::tasker::department_split::run(split_config) {
                    error!("Error running DepartmentSplit task #{}: {:?}", task_idx, e);
                    anyhow::bail!("DepartmentSplit task {} failed: {}", task_idx, e);
                }
                tracing::trace!("DepartmentSplit for task #{} finished.", task_idx);
            }
            TaskConfig::OpdAnalysis(opd_config) => {
                tracing::trace!("Executing OpdAnalysis for task #{}.", task_idx);
                if let Err(e) = opd_task::run(opd_config) {
                    error!("Error running OpdAnalysis task #{}: {:?}", task_idx, e);
                    anyhow::bail!("OpdAnalysis task {} failed: {}", task_idx, e);
                }
                tracing::trace!("OpdAnalysis for task #{} finished.", task_idx);
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
            AppArg::new("--config", ArgType::String),
            AppArg::new("--task", ArgType::List).options(vec![
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "4".to_string(),
                "5".to_string(),
            ]),
            AppArg::new("--only-call-center", ArgType::Boolean).depends_on(
                std::collections::HashMap::from([("--task".to_string(), vec!["1".to_string()])]),
            ),
            AppArg::new("--send-exceptions", ArgType::Boolean).depends_on(
                std::collections::HashMap::from([("--task".to_string(), vec!["1".to_string()])]),
            ),
        ],
    }
}

fn main() -> Result<()> {
    if let InterceptResult::ExitSuccessfully = intercept_manifest(get_manifest()) {
        return Ok(());
    }

    let options = TaskerCliOptions::parse();

    // Attempt early config load for logging levels
    let config_path = executable_dir()?.join("tasker_config.json");
    let (stdout_lvl, file_lvl) = if config_path.exists() {
        if let Ok(raw) = std::fs::read_to_string(&config_path) {
            if let Ok(cfg) = serde_json::from_str::<crm_tool::tasker::config::TaskerConfig>(&raw) {
                (cfg.log_stdout_level, cfg.log_file_level)
            } else {
                ("DEBUG".to_string(), "TRACE".to_string())
            }
        } else {
            ("DEBUG".to_string(), "TRACE".to_string())
        }
    } else {
        ("DEBUG".to_string(), "TRACE".to_string())
    };

    let _guard = setup_logging_with_levels(
        "task_csv_analysis",
        parse_log_level(&stdout_lvl)?,
        parse_log_level(&file_lvl)?,
    )?;

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

        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("mock_tasker_config_valid.json");

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
        drop(file);

        // Passing valid task index 2. This won't actually succeed all the way because 'path2' doesn't exist.
        // With correct error propagation, run_app should return Err now.
        let args = vec![
            "tasker".to_string(),
            "--config".to_string(),
            config_path.to_string_lossy().to_string(),
            "--task".to_string(),
            "2".to_string(),
        ];

        let options = TaskerCliOptions::parse_from(args);

        // We know it won't bail on BoundsCheck.
        // It will return Err(_) because path2 doesn't exist and we now propagate CsvAnalysis errors.
        let res = run_app(options);
        assert!(
            res.is_err(),
            "run_app MUST bail when the task index is valid but the task itself fails"
        );
        let err_msg = res.unwrap_err().to_string();
        assert!(
            err_msg.contains("CsvAnalysis task 2 failed"),
            "Error message should mention task failure"
        );
    }

    #[test]
    fn test_task_filtering_logic_out_of_bounds() {
        use std::io::Write;

        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("mock_tasker_config_oob.json");

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
        drop(file);

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
            ],
            "new_field": "default_value"
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

        // Test the shared crate merge_json which we now use via load_or_create_config
        let changed = crm_tool::utils::merge_json(&mut user_config, &default_config);

        assert!(
            changed,
            "Merge should mark config as changed because of new_field"
        );

        // Note: We updated merge_json to specifically recursively merge elements of the "tasks" array
        // (to allow auto-healing).
        let merged_task = &user_config["tasks"][0];
        assert_eq!(
            merged_task["minutes_ago"], 15,
            "Because tasks are now recursively merged, minutes_ago gets populated from default"
        );
        assert_eq!(
            merged_task["email_config"]["send_emails"], true,
            "Original array content preserved but merged with defaults"
        );
        assert_eq!(user_config["new_field"], "default_value");
    }

    #[test]
    fn test_empty_tasks_panics_on_start() {
        use std::io::Write;

        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("mock_tasker_config_empty.json");

        let mock_config_json = serde_json::json!({
            "tasks": []
        });

        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(mock_config_json.to_string().as_bytes())
            .unwrap();
        file.sync_all().unwrap();
        drop(file);

        let args = vec![
            "tasker".to_string(),
            "--config".to_string(),
            config_path.to_string_lossy().to_string(),
        ];

        let options = TaskerCliOptions::parse_from(args);

        // This validates the PR 1 characterization goal: empty tasks array behavior.
        // As noted in planning, the config panic is test-only. Production run_app should handle this gracefully by exiting with Ok(()) and doing nothing.
        let res = run_app(options);
        assert!(res.is_ok(), "run_app should successfully process 0 tasks");
    }
}
