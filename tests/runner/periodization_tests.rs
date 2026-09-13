use chrono::{Local, NaiveDate, TimeZone, Timelike, Utc};
use crm_tool::runner::config::RunnerConfig;
use crm_tool::runner::config::{
    generate_execution_periods, generate_upcoming_executions_for_app,
    resolve_and_generate_execution_periods, ActionSpec, ExecutionMode, ExternalAppSpec, PeriodMode,
    RunnerTask, TaskSchedule, TaskStep,
};
use std::collections::HashMap;

#[test]
fn test_external_app_spec_owns_periodization_and_dates() {
    let spec = ExternalAppSpec {
        app_id: "app1".to_string(),
        args: HashMap::new(),
        period_mode: PeriodMode::Monthly,
        start_date: Some("2026-01-01".to_string()),
        end_date: Some("2026-09-01".to_string()),
    };

    assert_eq!(spec.period_mode, PeriodMode::Monthly);
    assert_eq!(spec.start_date.as_deref(), Some("2026-01-01"));
    assert_eq!(spec.end_date.as_deref(), Some("2026-09-01"));
}

#[test]
fn test_sequential_date_resolution_next_weekday() {
    let now = Utc.with_ymd_and_hms(2026, 9, 8, 12, 0, 0).unwrap(); // Tuesday Sep 8, 2026

    // Resolve start: next sat -> Saturday Sep 12, 2026
    // Resolve end: next sat using start as base -> Saturday Sep 12, 2026 (exact same Saturday!)
    let periods = resolve_and_generate_execution_periods(
        PeriodMode::Custom,
        Some("next sat"),
        Some("next sat"),
        now,
    )
    .unwrap();

    assert_eq!(periods.len(), 1);
    assert_eq!(
        periods[0].start_date,
        NaiveDate::from_ymd_opt(2026, 9, 12).unwrap()
    );
    assert_eq!(
        periods[0].end_date,
        NaiveDate::from_ymd_opt(2026, 9, 12).unwrap()
    );
}

#[test]
fn test_beginning_of_month_and_eomonth_resolution() {
    let now = Utc.with_ymd_and_hms(2026, 2, 10, 12, 0, 0).unwrap();

    let periods = resolve_and_generate_execution_periods(
        PeriodMode::Custom,
        Some("beginning_of_month"),
        Some("eomonth"),
        now,
    )
    .unwrap();

    assert_eq!(periods.len(), 1);
    assert_eq!(
        periods[0].start_date,
        NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()
    );
    assert_eq!(
        periods[0].end_date,
        NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
    );
}

#[test]
fn test_invalid_date_expression_fails_explicitly() {
    let now = Utc::now();
    assert!(resolve_and_generate_execution_periods(
        PeriodMode::Custom,
        Some("invalid_date_xyz"),
        Some("2026-12-31"),
        now
    )
    .is_err());

    assert!(resolve_and_generate_execution_periods(
        PeriodMode::Custom,
        Some("2026-01-01"),
        Some("invalid_end_xyz"),
        now
    )
    .is_err());
}

#[test]
fn test_custom_range_validation_start_after_end() {
    let now = Utc::now();
    let res = resolve_and_generate_execution_periods(
        PeriodMode::Custom,
        Some("2026-12-31"),
        Some("2026-01-01"),
        now,
    );
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("is after end date"));
}

#[test]
fn test_period_generation_semantics() {
    let jan15 = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
    let sep10 = NaiveDate::from_ymd_opt(2026, 9, 10).unwrap();

    // Monthly
    let monthly = generate_execution_periods(PeriodMode::Monthly, jan15, sep10);
    assert_eq!(monthly.len(), 9);
    assert_eq!(
        monthly[0].start_date,
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
    );
    assert_eq!(
        monthly[0].end_date,
        NaiveDate::from_ymd_opt(2026, 1, 31).unwrap()
    );
    assert_eq!(
        monthly[8].start_date,
        NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()
    );
    assert_eq!(
        monthly[8].end_date,
        NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()
    );

    // Quarterly
    let feb15 = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
    let aug10 = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();
    let quarterly = generate_execution_periods(PeriodMode::Quarterly, feb15, aug10);
    assert_eq!(quarterly.len(), 3);
    assert_eq!(
        quarterly[0].start_date,
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
    );
    assert_eq!(
        quarterly[0].end_date,
        NaiveDate::from_ymd_opt(2026, 3, 31).unwrap()
    );
    assert_eq!(
        quarterly[2].start_date,
        NaiveDate::from_ymd_opt(2026, 7, 1).unwrap()
    );
    assert_eq!(
        quarterly[2].end_date,
        NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()
    );

    // A Month
    let start_2022 = NaiveDate::from_ymd_opt(2022, 9, 15).unwrap();
    let end_2026 = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
    let a_month = generate_execution_periods(PeriodMode::AMonth, start_2022, end_2026);
    assert_eq!(a_month.len(), 5);
    for (i, yr) in (2022..=2026).enumerate() {
        assert_eq!(
            a_month[i].start_date,
            NaiveDate::from_ymd_opt(yr, 9, 1).unwrap()
        );
        assert_eq!(
            a_month[i].end_date,
            NaiveDate::from_ymd_opt(yr, 9, 30).unwrap()
        );
    }

    // A Quarter
    let start_q3 = NaiveDate::from_ymd_opt(2022, 8, 15).unwrap();
    let a_quarter = generate_execution_periods(PeriodMode::AQuarter, start_q3, end_2026);
    assert_eq!(a_quarter.len(), 5);
    for (i, yr) in (2022..=2026).enumerate() {
        assert_eq!(
            a_quarter[i].start_date,
            NaiveDate::from_ymd_opt(yr, 7, 1).unwrap()
        );
        assert_eq!(
            a_quarter[i].end_date,
            NaiveDate::from_ymd_opt(yr, 9, 30).unwrap()
        );
    }
}

#[test]
fn test_multiple_external_apps_independence_and_preview() {
    let now = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();

    let app_a = ExternalAppSpec {
        app_id: "app_a".to_string(),
        args: HashMap::new(),
        period_mode: PeriodMode::Monthly,
        start_date: Some("2026-01-01".to_string()),
        end_date: Some("2026-03-31".to_string()),
    };

    let app_b = ExternalAppSpec {
        app_id: "app_b".to_string(),
        args: HashMap::new(),
        period_mode: PeriodMode::Quarterly,
        start_date: Some("2026-01-01".to_string()),
        end_date: Some("2026-12-31".to_string()),
    };

    let task = RunnerTask {
        id: "multi_app_task".to_string(),
        name: "Multi App Task".to_string(),
        enabled: true,
        repetition: crm_tool::runner::config::Repetition::Once,
        frequency_seconds: 0,
        next_run_at: String::new(),
        schedules: vec![],
        steps: vec![TaskStep {
            name: None,
            mode: ExecutionMode::Sequential,
            actions: vec![
                ActionSpec::ExternalApp(app_a.clone()),
                ActionSpec::ExternalApp(app_b.clone()),
            ],
        }],
        post_run_steps: vec![],
        last_run_at: String::new(),
        last_status: String::new(),
        timeout_seconds: 0,
    };

    let preview_a = generate_upcoming_executions_for_app(&task, &app_a, now, 10).unwrap();
    let preview_b = generate_upcoming_executions_for_app(&task, &app_b, now, 10).unwrap();

    // App A has 3 monthly periods (Jan, Feb, Mar)
    assert_eq!(preview_a.len(), 3);
    assert_eq!(
        preview_a[0].period.start_date,
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
    );
    assert_eq!(
        preview_a[2].period.start_date,
        NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()
    );

    // App B has 4 quarterly periods (Q1, Q2, Q3, Q4)
    assert_eq!(preview_b.len(), 4);
    assert_eq!(
        preview_b[0].period.start_date,
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
    );
    assert_eq!(
        preview_b[3].period.start_date,
        NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()
    );
}

#[test]
fn test_preview_interval_grid_alignment() {
    // Current time: 10:37 UTC
    let now = Utc.with_ymd_and_hms(2026, 1, 1, 10, 37, 0).unwrap();

    let app = ExternalAppSpec {
        app_id: "grid_app".to_string(),
        args: HashMap::new(),
        period_mode: PeriodMode::Custom,
        start_date: Some("2026-01-01".to_string()),
        end_date: Some("2026-01-01".to_string()),
    };

    // Configured start_time = 08:00, interval = 1 hour (3600s)
    let task = RunnerTask {
        id: "grid_task".to_string(),
        name: "Grid Task".to_string(),
        enabled: true,
        repetition: crm_tool::runner::config::Repetition::Repeat,
        frequency_seconds: 3600,
        next_run_at: String::new(),
        schedules: vec![TaskSchedule::Interval {
            enabled: true,
            every_seconds: 3600,
            next_run_at: String::new(),
            working_hours: None,
            working_hours_profile_id: None,
            start_time: Some("08:00".to_string()),
        }],
        steps: vec![],
        post_run_steps: vec![],
        last_run_at: String::new(),
        last_status: String::new(),
        timeout_seconds: 0,
    };

    let preview = generate_upcoming_executions_for_app(&task, &app, now, 10).unwrap();
    assert!(!preview.is_empty());

    // Next scheduled execution MUST be 11:00 UTC (aligned to 08:00 grid), NOT 10:37!
    let first = preview[0].scheduled_at;
    let local_first = first.with_timezone(&Local);
    assert_eq!(local_first.hour(), 11);
    assert_eq!(local_first.minute(), 0);
}

fn load_config_from_json(json_str: &str) -> (RunnerConfig, tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("runner_config.json");
    std::fs::write(&file_path, json_str).unwrap();
    let cfg = RunnerConfig::load(file_path.to_str().unwrap()).unwrap();
    (cfg, dir, file_path)
}

#[test]
fn test_loader_migration_a_legacy_inheritance() {
    let json = r#"{
        "tasks": [{
            "id": "task_a",
            "name": "Task A",
            "enabled": true,
            "repetition": "once",
            "frequency_seconds": 0,
            "next_run_at": "",
            "schedules": [],
            "steps": [{
                "mode": "sequential",
                "actions": [{ "type": "external_app", "app_id": "app_a", "args": {} }]
            }],
            "post_run_steps": [],
            "last_run_at": "",
            "last_status": "",
            "timeout_seconds": 0,
            "period_mode": "monthly",
            "start_date": "2026-01-01",
            "end_date": "2026-09-30"
        }]
    }"#;

    let (cfg, _dir, _path) = load_config_from_json(json);
    assert_eq!(cfg.tasks.len(), 1);
    if let ActionSpec::ExternalApp(ref spec) = cfg.tasks[0].steps[0].actions[0] {
        assert_eq!(spec.period_mode, PeriodMode::Monthly);
        assert_eq!(spec.start_date.as_deref(), Some("2026-01-01"));
        assert_eq!(spec.end_date.as_deref(), Some("2026-09-30"));
    } else {
        panic!("Expected ExternalApp");
    }
}

#[test]
fn test_loader_migration_b_explicit_custom_must_win() {
    let json = r#"{
        "tasks": [{
            "id": "task_b",
            "name": "Task B",
            "enabled": true,
            "repetition": "once",
            "frequency_seconds": 0,
            "next_run_at": "",
            "schedules": [],
            "steps": [{
                "mode": "sequential",
                "actions": [{
                    "type": "external_app",
                    "app_id": "app_b",
                    "args": {},
                    "period_mode": "custom"
                }]
            }],
            "post_run_steps": [],
            "last_run_at": "",
            "last_status": "",
            "timeout_seconds": 0,
            "period_mode": "monthly"
        }]
    }"#;

    let (cfg, _dir, _path) = load_config_from_json(json);
    if let ActionSpec::ExternalApp(ref spec) = cfg.tasks[0].steps[0].actions[0] {
        assert_eq!(spec.period_mode, PeriodMode::Custom);
    } else {
        panic!("Expected ExternalApp");
    }
}

#[test]
fn test_loader_migration_c_explicit_dates_must_win() {
    let json = r#"{
        "tasks": [{
            "id": "task_c",
            "name": "Task C",
            "enabled": true,
            "repetition": "once",
            "frequency_seconds": 0,
            "next_run_at": "",
            "schedules": [],
            "steps": [{
                "mode": "sequential",
                "actions": [{
                    "type": "external_app",
                    "app_id": "app_c",
                    "args": {},
                    "start_date": "2026-02-01",
                    "end_date": "2026-02-28"
                }]
            }],
            "post_run_steps": [],
            "last_run_at": "",
            "last_status": "",
            "timeout_seconds": 0,
            "start_date": "2026-01-01",
            "end_date": "2026-09-30"
        }]
    }"#;

    let (cfg, _dir, _path) = load_config_from_json(json);
    if let ActionSpec::ExternalApp(ref spec) = cfg.tasks[0].steps[0].actions[0] {
        assert_eq!(spec.start_date.as_deref(), Some("2026-02-01"));
        assert_eq!(spec.end_date.as_deref(), Some("2026-02-28"));
    } else {
        panic!("Expected ExternalApp");
    }
}

#[test]
fn test_loader_migration_d_mixed_inheritance() {
    let json = r#"{
        "tasks": [{
            "id": "task_d",
            "name": "Task D",
            "enabled": true,
            "repetition": "once",
            "frequency_seconds": 0,
            "next_run_at": "",
            "schedules": [],
            "steps": [{
                "mode": "sequential",
                "actions": [{
                    "type": "external_app",
                    "app_id": "app_d",
                    "args": {},
                    "period_mode": "custom",
                    "end_date": "2026-03-31"
                }]
            }],
            "post_run_steps": [],
            "last_run_at": "",
            "last_status": "",
            "timeout_seconds": 0,
            "period_mode": "monthly",
            "start_date": "2026-01-01",
            "end_date": "2026-09-30"
        }]
    }"#;

    let (cfg, _dir, _path) = load_config_from_json(json);
    if let ActionSpec::ExternalApp(ref spec) = cfg.tasks[0].steps[0].actions[0] {
        assert_eq!(spec.period_mode, PeriodMode::Custom);
        assert_eq!(spec.start_date.as_deref(), Some("2026-01-01"));
        assert_eq!(spec.end_date.as_deref(), Some("2026-03-31"));
    } else {
        panic!("Expected ExternalApp");
    }
}

#[test]
fn test_loader_migration_e_post_run_steps() {
    let json = r#"{
        "tasks": [{
            "id": "task_e",
            "name": "Task E",
            "enabled": true,
            "repetition": "once",
            "frequency_seconds": 0,
            "next_run_at": "",
            "schedules": [],
            "steps": [],
            "post_run_steps": [{
                "mode": "sequential",
                "actions": [{
                    "type": "external_app",
                    "app_id": "app_e_1",
                    "args": {}
                }, {
                    "type": "external_app",
                    "app_id": "app_e_2",
                    "args": {},
                    "period_mode": "custom"
                }]
            }],
            "last_run_at": "",
            "last_status": "",
            "timeout_seconds": 0,
            "period_mode": "monthly",
            "start_date": "2026-01-01",
            "end_date": "2026-09-30"
        }]
    }"#;

    let (cfg, _dir, _path) = load_config_from_json(json);
    if let ActionSpec::ExternalApp(ref spec1) = cfg.tasks[0].post_run_steps[0].actions[0] {
        assert_eq!(spec1.period_mode, PeriodMode::Monthly);
        assert_eq!(spec1.start_date.as_deref(), Some("2026-01-01"));
        assert_eq!(spec1.end_date.as_deref(), Some("2026-09-30"));
    } else {
        panic!("Expected ExternalApp");
    }

    if let ActionSpec::ExternalApp(ref spec2) = cfg.tasks[0].post_run_steps[0].actions[1] {
        assert_eq!(spec2.period_mode, PeriodMode::Custom);
        assert_eq!(spec2.start_date.as_deref(), Some("2026-01-01"));
        assert_eq!(spec2.end_date.as_deref(), Some("2026-09-30"));
    } else {
        panic!("Expected ExternalApp");
    }
}

#[test]
fn test_loader_migration_f_multiple_apps_independence() {
    let json = r#"{
        "tasks": [{
            "id": "task_f",
            "name": "Task F",
            "enabled": true,
            "repetition": "once",
            "frequency_seconds": 0,
            "next_run_at": "",
            "schedules": [],
            "steps": [{
                "mode": "sequential",
                "actions": [
                    { "type": "external_app", "app_id": "app_f_1", "args": {} },
                    {
                        "type": "external_app",
                        "app_id": "app_f_2",
                        "args": {},
                        "period_mode": "quarterly",
                        "start_date": "2026-02-01",
                        "end_date": "2026-06-30"
                    }
                ]
            }],
            "post_run_steps": [],
            "last_run_at": "",
            "last_status": "",
            "timeout_seconds": 0,
            "period_mode": "monthly",
            "start_date": "2026-01-01",
            "end_date": "2026-09-30"
        }]
    }"#;

    let (cfg, _dir, _path) = load_config_from_json(json);
    if let ActionSpec::ExternalApp(ref spec1) = cfg.tasks[0].steps[0].actions[0] {
        assert_eq!(spec1.period_mode, PeriodMode::Monthly);
        assert_eq!(spec1.start_date.as_deref(), Some("2026-01-01"));
        assert_eq!(spec1.end_date.as_deref(), Some("2026-09-30"));
    }
    if let ActionSpec::ExternalApp(ref spec2) = cfg.tasks[0].steps[0].actions[1] {
        assert_eq!(spec2.period_mode, PeriodMode::Quarterly);
        assert_eq!(spec2.start_date.as_deref(), Some("2026-02-01"));
        assert_eq!(spec2.end_date.as_deref(), Some("2026-06-30"));
    }
}

#[test]
fn test_loader_migration_g_save_and_reload_roundtrip() {
    let json = r#"{
        "tasks": [{
            "id": "task_g",
            "name": "Task G",
            "enabled": true,
            "repetition": "once",
            "frequency_seconds": 0,
            "next_run_at": "",
            "schedules": [],
            "steps": [{
                "mode": "sequential",
                "actions": [{
                    "type": "external_app",
                    "app_id": "app_g",
                    "args": {},
                    "period_mode": "custom"
                }]
            }],
            "post_run_steps": [],
            "last_run_at": "",
            "last_status": "",
            "timeout_seconds": 0,
            "period_mode": "monthly"
        }]
    }"#;

    let (cfg, _dir, file_path) = load_config_from_json(json);
    let path_str = file_path.to_str().unwrap().to_string();

    // Save migrated config back to file
    cfg.save(&path_str).expect("Failed to save config");

    // Read saved file content to verify task-level period/date fields are NOT persisted
    let file_content = std::fs::read_to_string(&path_str).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&file_content).unwrap();
    let task_obj = &json_val["tasks"][0];

    assert!(task_obj.get("period_mode").is_none());
    assert!(task_obj.get("start_date").is_none());
    assert!(task_obj.get("end_date").is_none());

    // Reload saved file and verify explicit Custom period_mode survives
    let reloaded_cfg = RunnerConfig::load(&path_str).unwrap();
    if let ActionSpec::ExternalApp(ref spec) = reloaded_cfg.tasks[0].steps[0].actions[0] {
        assert_eq!(spec.period_mode, PeriodMode::Custom);
    } else {
        panic!("Expected ExternalApp");
    }
}

#[tokio::test]
async fn test_concurrent_period_execution() {
    use crm_tool::runner::config::RegisteredApp;
    use crm_tool::runner::engine::pipeline::run_task_inner;
    use crm_tool::runner::engine::state::RunnerStatus;
    use crm_tool::runner::engine::{AppLockManager, ExecutionPolicy};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let app_concurrent = RegisteredApp {
        id: "concurrent_app".to_string(),
        name: "Concurrent App".to_string(),
        executable_path: "echo".to_string(),
        config_path: String::new(),
        allow_concurrent_tasks: true,
    };

    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 5,
        post_run_timeout_seconds: 5,
        min_task_interval_seconds: 1,
        registered_apps: vec![app_concurrent],
        log_retention_days: 30,
    };

    let spec = ExternalAppSpec {
        app_id: "concurrent_app".to_string(),
        args: HashMap::new(),
        period_mode: PeriodMode::Monthly,
        start_date: Some("2026-01-01".to_string()),
        end_date: Some("2026-03-31".to_string()),
    };

    let mut task = RunnerTask {
        id: "concurrent_task".to_string(),
        name: "Concurrent Task".to_string(),
        enabled: true,
        repetition: crm_tool::runner::config::Repetition::Once,
        frequency_seconds: 0,
        next_run_at: String::new(),
        schedules: vec![],
        steps: vec![TaskStep {
            name: None,
            mode: ExecutionMode::Sequential,
            actions: vec![ActionSpec::ExternalApp(spec)],
        }],
        post_run_steps: vec![],
        last_run_at: String::new(),
        last_status: String::new(),
        timeout_seconds: 0,
    };

    let status = Arc::new(Mutex::new(RunnerStatus {
        running_tasks_count: 0,
        queued_tasks_count: 0,
        running_task_ids: Vec::new(),
        queued_task_ids: Vec::new(),
        last_error: String::new(),
        last_task_id: String::new(),
        last_run_at: String::new(),
        waiting_for_app: HashMap::new(),
    }));
    let app_lock_mgr = AppLockManager::new();

    let res = run_task_inner(&mut task, &policy, &status, &app_lock_mgr).await;

    assert!(res.success);
    assert_eq!(task.last_status, "ok");
}

#[test]
fn test_beginning_of_prev_month_resolution() {
    let now = Utc.with_ymd_and_hms(2026, 3, 15, 12, 0, 0).unwrap(); // March 15, 2026

    let periods = resolve_and_generate_execution_periods(
        PeriodMode::Custom,
        Some("beginning_of_prev_month"),
        Some("eomonth"),
        now,
    )
    .unwrap();

    assert_eq!(periods.len(), 1);
    // Start Date: Feb 1, 2026
    assert_eq!(
        periods[0].start_date,
        NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()
    );
    // End Date (eomonth based on Feb 1): Feb 28, 2026
    assert_eq!(
        periods[0].end_date,
        NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
    );
}

#[tokio::test]
async fn test_concurrent_period_execution_overlapping() {
    use crm_tool::runner::config::RegisteredApp;
    use crm_tool::runner::engine::pipeline::run_task_inner;
    use crm_tool::runner::engine::state::RunnerStatus;
    use crm_tool::runner::engine::{AppLockManager, ExecutionPolicy};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let temp_dir = tempfile::tempdir().unwrap();
    let log_file = temp_dir.path().join("execution_log.txt");
    let log_path_str = log_file.to_str().unwrap().replace("\\", "/");

    // Python test script that records start/finish times and argument values
    let py_script = format!(
        "import sys, time; args = ' '.join(sys.argv); f = open('{}', 'a'); f.write('START ' + args + chr(10)); f.close(); time.sleep(0.15); f = open('{}', 'a'); f.write('FINISH ' + args + chr(10)); f.close()",
        log_path_str, log_path_str
    );

    let app_concurrent = RegisteredApp {
        id: "concurrent_app".to_string(),
        name: "Concurrent App".to_string(),
        executable_path: "python3".to_string(),
        config_path: String::new(),
        allow_concurrent_tasks: true,
    };

    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 5,
        post_run_timeout_seconds: 5,
        min_task_interval_seconds: 1,
        registered_apps: vec![app_concurrent],
        log_retention_days: 30,
    };

    let mut args = HashMap::new();
    args.insert("-c".to_string(), py_script);

    let spec = ExternalAppSpec {
        app_id: "concurrent_app".to_string(),
        args,
        period_mode: PeriodMode::Monthly,
        start_date: Some("2026-01-01".to_string()),
        end_date: Some("2026-02-28".to_string()),
    };

    let mut task = RunnerTask {
        id: "concurrent_task".to_string(),
        name: "Concurrent Task".to_string(),
        enabled: true,
        repetition: crm_tool::runner::config::Repetition::Once,
        frequency_seconds: 0,
        next_run_at: String::new(),
        schedules: vec![],
        steps: vec![TaskStep {
            name: None,
            mode: ExecutionMode::Sequential,
            actions: vec![ActionSpec::ExternalApp(spec)],
        }],
        post_run_steps: vec![],
        last_run_at: String::new(),
        last_status: String::new(),
        timeout_seconds: 0,
    };

    let status = Arc::new(Mutex::new(RunnerStatus {
        running_tasks_count: 0,
        queued_tasks_count: 0,
        running_task_ids: Vec::new(),
        queued_task_ids: Vec::new(),
        last_error: String::new(),
        last_task_id: String::new(),
        last_run_at: String::new(),
        waiting_for_app: HashMap::new(),
    }));
    let app_lock_mgr = AppLockManager::new();

    let res = run_task_inner(&mut task, &policy, &status, &app_lock_mgr).await;

    assert!(res.success);
    assert_eq!(task.last_status, "ok");

    let log_content = std::fs::read_to_string(&log_file).unwrap_or_default();
    let lines: Vec<&str> = log_content.lines().collect();

    // Verify both Period 1 (2026-01-01 -> 2026-01-31) and Period 2 (2026-02-01 -> 2026-02-28) executed
    assert_eq!(lines.len(), 4, "Log output: \n{}", log_content);

    // Concurrency check: Period 2 START occurs before Period 1 FINISH!
    assert!(lines[0].starts_with("START"));
    assert!(
        lines[1].starts_with("START"),
        "Second line should be START for concurrent execution: {}",
        log_content
    );
    assert!(lines[2].starts_with("FINISH"));
    assert!(lines[3].starts_with("FINISH"));

    // Verify date substitution in arguments
    assert!(log_content.contains("2026-01-01"));
    assert!(log_content.contains("2026-01-31"));
    assert!(log_content.contains("2026-02-01"));
    assert!(log_content.contains("2026-02-28"));
}

#[tokio::test]
async fn test_sequential_period_execution() {
    use crm_tool::runner::config::RegisteredApp;
    use crm_tool::runner::engine::pipeline::run_task_inner;
    use crm_tool::runner::engine::state::RunnerStatus;
    use crm_tool::runner::engine::{AppLockManager, ExecutionPolicy};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let temp_dir = tempfile::tempdir().unwrap();
    let log_file = temp_dir.path().join("seq_log.txt");
    let log_path_str = log_file.to_str().unwrap().replace("\\", "/");

    let py_script = format!(
        "import sys, time; args = ' '.join(sys.argv); f = open('{}', 'a'); f.write('START ' + args + chr(10)); f.close(); time.sleep(0.05); f = open('{}', 'a'); f.write('FINISH ' + args + chr(10)); f.close()",
        log_path_str, log_path_str
    );

    let app_seq = RegisteredApp {
        id: "seq_app".to_string(),
        name: "Sequential App".to_string(),
        executable_path: "python3".to_string(),
        config_path: String::new(),
        allow_concurrent_tasks: false,
    };

    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 5,
        post_run_timeout_seconds: 5,
        min_task_interval_seconds: 1,
        registered_apps: vec![app_seq],
        log_retention_days: 30,
    };

    let mut args = HashMap::new();
    args.insert("-c".to_string(), py_script);

    let spec = ExternalAppSpec {
        app_id: "seq_app".to_string(),
        args,
        period_mode: PeriodMode::Monthly,
        start_date: Some("2026-01-01".to_string()),
        end_date: Some("2026-02-28".to_string()),
    };

    let mut task = RunnerTask {
        id: "seq_task".to_string(),
        name: "Sequential Task".to_string(),
        enabled: true,
        repetition: crm_tool::runner::config::Repetition::Once,
        frequency_seconds: 0,
        next_run_at: String::new(),
        schedules: vec![],
        steps: vec![TaskStep {
            name: None,
            mode: ExecutionMode::Sequential,
            actions: vec![ActionSpec::ExternalApp(spec)],
        }],
        post_run_steps: vec![],
        last_run_at: String::new(),
        last_status: String::new(),
        timeout_seconds: 0,
    };

    let status = Arc::new(Mutex::new(RunnerStatus {
        running_tasks_count: 0,
        queued_tasks_count: 0,
        running_task_ids: Vec::new(),
        queued_task_ids: Vec::new(),
        last_error: String::new(),
        last_task_id: String::new(),
        last_run_at: String::new(),
        waiting_for_app: HashMap::new(),
    }));
    let app_lock_mgr = AppLockManager::new();

    let res = run_task_inner(&mut task, &policy, &status, &app_lock_mgr).await;

    assert!(res.success);

    let log_content = std::fs::read_to_string(&log_file).unwrap_or_default();
    let lines: Vec<&str> = log_content.lines().collect();

    assert_eq!(lines.len(), 4, "Log output: \n{}", log_content);

    // Sequential check: Period 1 START -> Period 1 FINISH -> Period 2 START -> Period 2 FINISH
    assert!(lines[0].starts_with("START"));
    assert!(
        lines[1].starts_with("FINISH"),
        "Line 1 must be FINISH for sequential execution: {}",
        log_content
    );
    assert!(lines[2].starts_with("START"));
    assert!(lines[3].starts_with("FINISH"));
}

#[tokio::test]
async fn test_concurrent_period_error_propagation() {
    use crm_tool::runner::config::RegisteredApp;
    use crm_tool::runner::engine::pipeline::run_task_inner;
    use crm_tool::runner::engine::state::RunnerStatus;
    use crm_tool::runner::engine::{AppLockManager, ExecutionPolicy};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let app_failing = RegisteredApp {
        id: "failing_app".to_string(),
        name: "Failing App".to_string(),
        executable_path: "python3".to_string(),
        config_path: String::new(),
        allow_concurrent_tasks: true,
    };

    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 5,
        post_run_timeout_seconds: 5,
        min_task_interval_seconds: 1,
        registered_apps: vec![app_failing],
        log_retention_days: 30,
    };

    let mut args = HashMap::new();
    args.insert("-c".to_string(), "import sys; sys.exit(1)".to_string());

    let spec = ExternalAppSpec {
        app_id: "failing_app".to_string(),
        args,
        period_mode: PeriodMode::Monthly,
        start_date: Some("2026-01-01".to_string()),
        end_date: Some("2026-02-28".to_string()),
    };

    let mut task = RunnerTask {
        id: "failing_task".to_string(),
        name: "Failing Task".to_string(),
        enabled: true,
        repetition: crm_tool::runner::config::Repetition::Once,
        frequency_seconds: 0,
        next_run_at: String::new(),
        schedules: vec![],
        steps: vec![TaskStep {
            name: None,
            mode: ExecutionMode::Sequential,
            actions: vec![ActionSpec::ExternalApp(spec)],
        }],
        post_run_steps: vec![],
        last_run_at: String::new(),
        last_status: String::new(),
        timeout_seconds: 0,
    };

    let status = Arc::new(Mutex::new(RunnerStatus {
        running_tasks_count: 0,
        queued_tasks_count: 0,
        running_task_ids: Vec::new(),
        queued_task_ids: Vec::new(),
        last_error: String::new(),
        last_task_id: String::new(),
        last_run_at: String::new(),
        waiting_for_app: HashMap::new(),
    }));
    let app_lock_mgr = AppLockManager::new();

    let res = run_task_inner(&mut task, &policy, &status, &app_lock_mgr).await;
    assert!(!res.success);
    assert!(task.last_status.contains("error"));
}

#[tokio::test]
async fn test_multiple_external_apps_execution_isolation() {
    use crm_tool::runner::config::RegisteredApp;
    use crm_tool::runner::engine::pipeline::run_task_inner;
    use crm_tool::runner::engine::state::RunnerStatus;
    use crm_tool::runner::engine::{AppLockManager, ExecutionPolicy};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let temp_dir = tempfile::tempdir().unwrap();
    let log_file = temp_dir.path().join("iso_log.txt");
    let log_path_str = log_file.to_str().unwrap().replace("\\", "/");

    let py_script = format!(
        "import sys; args = ' '.join(sys.argv); f = open('{}', 'a'); f.write(args + chr(10)); f.close()",
        log_path_str
    );

    let app_iso = RegisteredApp {
        id: "iso_app".to_string(),
        name: "Iso App".to_string(),
        executable_path: "python3".to_string(),
        config_path: String::new(),
        allow_concurrent_tasks: false,
    };

    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 5,
        post_run_timeout_seconds: 5,
        min_task_interval_seconds: 1,
        registered_apps: vec![app_iso],
        log_retention_days: 30,
    };

    let mut args_a = HashMap::new();
    args_a.insert("-c".to_string(), py_script.clone());

    let app_a_spec = ExternalAppSpec {
        app_id: "iso_app".to_string(),
        args: args_a,
        period_mode: PeriodMode::Monthly,
        start_date: Some("2026-01-01".to_string()),
        end_date: Some("2026-01-31".to_string()),
    };

    let mut args_b = HashMap::new();
    args_b.insert("-c".to_string(), py_script);

    let app_b_spec = ExternalAppSpec {
        app_id: "iso_app".to_string(),
        args: args_b,
        period_mode: PeriodMode::Quarterly,
        start_date: Some("2026-04-01".to_string()),
        end_date: Some("2026-06-30".to_string()),
    };

    let mut task = RunnerTask {
        id: "iso_task".to_string(),
        name: "Iso Task".to_string(),
        enabled: true,
        repetition: crm_tool::runner::config::Repetition::Once,
        frequency_seconds: 0,
        next_run_at: String::new(),
        schedules: vec![],
        steps: vec![TaskStep {
            name: None,
            mode: ExecutionMode::Sequential,
            actions: vec![
                ActionSpec::ExternalApp(app_a_spec),
                ActionSpec::ExternalApp(app_b_spec),
            ],
        }],
        post_run_steps: vec![],
        last_run_at: String::new(),
        last_status: String::new(),
        timeout_seconds: 0,
    };

    let status = Arc::new(Mutex::new(RunnerStatus {
        running_tasks_count: 0,
        queued_tasks_count: 0,
        running_task_ids: Vec::new(),
        queued_task_ids: Vec::new(),
        last_error: String::new(),
        last_task_id: String::new(),
        last_run_at: String::new(),
        waiting_for_app: HashMap::new(),
    }));
    let app_lock_mgr = AppLockManager::new();

    let res = run_task_inner(&mut task, &policy, &status, &app_lock_mgr).await;

    assert!(res.success);

    let log_content = std::fs::read_to_string(&log_file).unwrap_or_default();
    assert!(log_content.contains("2026-01-01"));
    assert!(log_content.contains("2026-01-31"));
    assert!(log_content.contains("2026-04-01"));
    assert!(log_content.contains("2026-06-30"));
}

#[tokio::test]
async fn test_post_run_external_app_execution() {
    use crm_tool::runner::config::RegisteredApp;
    use crm_tool::runner::engine::pipeline::run_task_inner;
    use crm_tool::runner::engine::state::RunnerStatus;
    use crm_tool::runner::engine::{AppLockManager, ExecutionPolicy};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let temp_dir = tempfile::tempdir().unwrap();
    let log_file = temp_dir.path().join("post_log.txt");
    let log_path_str = log_file.to_str().unwrap().replace("\\", "/");

    let py_script = format!(
        "import sys; args = ' '.join(sys.argv); f = open('{}', 'a'); f.write(args + chr(10)); f.close()",
        log_path_str
    );

    let app_post = RegisteredApp {
        id: "post_app".to_string(),
        name: "Post App".to_string(),
        executable_path: "python3".to_string(),
        config_path: String::new(),
        allow_concurrent_tasks: false,
    };

    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 5,
        post_run_timeout_seconds: 5,
        min_task_interval_seconds: 1,
        registered_apps: vec![app_post],
        log_retention_days: 30,
    };

    let mut args_post = HashMap::new();
    args_post.insert("-c".to_string(), py_script);

    let post_app_spec = ExternalAppSpec {
        app_id: "post_app".to_string(),
        args: args_post,
        period_mode: PeriodMode::Monthly,
        start_date: Some("2026-05-01".to_string()),
        end_date: Some("2026-05-31".to_string()),
    };

    let mut task = RunnerTask {
        id: "post_task".to_string(),
        name: "Post Task".to_string(),
        enabled: true,
        repetition: crm_tool::runner::config::Repetition::Once,
        frequency_seconds: 0,
        next_run_at: String::new(),
        schedules: vec![],
        steps: vec![],
        post_run_steps: vec![TaskStep {
            name: None,
            mode: ExecutionMode::Sequential,
            actions: vec![ActionSpec::ExternalApp(post_app_spec)],
        }],
        last_run_at: String::new(),
        last_status: String::new(),
        timeout_seconds: 0,
    };

    let status = Arc::new(Mutex::new(RunnerStatus {
        running_tasks_count: 0,
        queued_tasks_count: 0,
        running_task_ids: Vec::new(),
        queued_task_ids: Vec::new(),
        last_error: String::new(),
        last_task_id: String::new(),
        last_run_at: String::new(),
        waiting_for_app: HashMap::new(),
    }));
    let app_lock_mgr = AppLockManager::new();

    let res = run_task_inner(&mut task, &policy, &status, &app_lock_mgr).await;

    assert!(res.success);

    let log_content = std::fs::read_to_string(&log_file).unwrap_or_default();
    assert!(log_content.contains("2026-05-01"));
    assert!(log_content.contains("2026-05-31"));
}
