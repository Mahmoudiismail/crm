use chrono::Utc;
use crm_tool::runner::config::models::*;
use std::fs;

#[test]
fn test_interval_schedule_persistence() {
    let task = RunnerTask {
        id: "test_interval".to_string(),
        name: "Test Interval Task".to_string(),
        enabled: true,
        repetition: Repetition::Repeat,
        frequency_seconds: 3600,
        next_run_at: String::new(),
        schedules: vec![TaskSchedule::Interval {
            enabled: true,
            every_seconds: 7200,
            next_run_at: Utc::now().to_rfc3339(),
            working_hours: None,
            working_hours_profile_id: None,
            start_time: None,
        }],
        steps: Vec::new(),
        post_run_steps: Vec::new(),
        last_run_at: String::new(),
        last_status: String::new(),
        timeout_seconds: 0,
    };

    let cfg = RunnerConfig {
        tasks: vec![task],
        ..Default::default()
    };

    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join("test_interval_config.json");
    cfg.save(&path.to_string_lossy()).unwrap();

    let loaded = RunnerConfig::load(&path.to_string_lossy()).unwrap();
    let _ = fs::remove_file(&path);

    assert_eq!(loaded.tasks.len(), 1);
    let loaded_task = loaded.tasks.first().expect("No tasks loaded");
    assert_eq!(loaded_task.id, "test_interval");
    assert_eq!(loaded_task.name, "Test Interval Task");
    assert!(loaded_task.enabled);

    assert_eq!(loaded_task.schedules.len(), 1);
    match loaded_task.schedules.first().expect("No schedules loaded") {
        TaskSchedule::Interval {
            every_seconds,
            enabled,
            ..
        } => {
            assert_eq!(*every_seconds, 7200);
            assert!(*enabled);
        }
        _ => panic!("Expected Interval schedule"),
    }
}

#[test]
fn test_shell_command_persistence() {
    let task = RunnerTask {
        id: "test_shell".to_string(),
        name: "Test Shell Task".to_string(),
        enabled: true,
        repetition: Repetition::Once,
        frequency_seconds: 0,
        next_run_at: String::new(),
        schedules: vec![],
        last_run_at: String::new(),
        steps: vec![TaskStep {
            name: None,
            mode: ExecutionMode::Parallel,
            actions: vec![
                ActionSpec::ShellCommand(ShellCommandSpec {
                    command: "tar -czf backup.tar.gz /data".to_string(),
                    continue_on_error: false,
                }),
                ActionSpec::ShellCommand(ShellCommandSpec {
                    command: "echo Backup complete".to_string(),
                    continue_on_error: true,
                }),
            ],
        }],
        post_run_steps: Vec::new(),
        last_status: String::new(),
        timeout_seconds: 0,
    };

    let cfg = RunnerConfig {
        tasks: vec![task],
        ..Default::default()
    };

    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join("test_shell_config.json");
    cfg.save(&path.to_string_lossy()).unwrap();

    let loaded = RunnerConfig::load(&path.to_string_lossy()).unwrap();
    let _ = fs::remove_file(&path);

    assert_eq!(loaded.tasks.len(), 1);
    let loaded_task = loaded.tasks.first().expect("No tasks loaded");
    assert_eq!(loaded_task.id, "test_shell");
    assert_eq!(loaded_task.name, "Test Shell Task");
    assert_eq!(loaded_task.steps.len(), 1);

    let step = &loaded_task.steps[0];
    assert_eq!(step.mode, ExecutionMode::Parallel);
    assert_eq!(step.actions.len(), 2);

    if let ActionSpec::ShellCommand(cmd) = &step.actions[0] {
        assert_eq!(cmd.command, "tar -czf backup.tar.gz /data");
        assert!(!cmd.continue_on_error);
    } else {
        panic!("Expected ShellCommand");
    }
}

#[test]
fn test_mixed_tasks_persistence() {
    let tasks = vec![
        RunnerTask {
            id: "crm_task".to_string(),
            name: "CRM Fetch".to_string(),
            enabled: true,
            repetition: Repetition::Repeat,
            frequency_seconds: 86400,
            next_run_at: String::new(),
            schedules: vec![TaskSchedule::Interval {
                enabled: true,
                every_seconds: 86400,
                next_run_at: Utc::now().to_rfc3339(),
                working_hours: None,
                working_hours_profile_id: None,
                start_time: None,
            }],
            steps: Vec::new(),
            post_run_steps: Vec::new(),
            last_run_at: String::new(),
            last_status: String::new(),
            timeout_seconds: 0,
        },
        RunnerTask {
            id: "shell_task".to_string(),
            name: "Shell Commands".to_string(),
            enabled: false,
            repetition: Repetition::Once,
            frequency_seconds: 0,
            next_run_at: String::new(),
            schedules: vec![TaskSchedule::Once {
                enabled: true,
                next_run_at: (Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
            }],
            steps: vec![TaskStep {
                name: None,
                mode: ExecutionMode::Sequential,
                actions: vec![ActionSpec::ShellCommand(ShellCommandSpec {
                    command: "echo Hello World".to_string(),
                    continue_on_error: false,
                })],
            }],
            post_run_steps: Vec::new(),
            last_run_at: String::new(),
            last_status: String::new(),
            timeout_seconds: 0,
        },
    ];

    let cfg = RunnerConfig {
        tasks: tasks.clone(),
        ..Default::default()
    };

    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join("mixed_config.json");
    cfg.save(&path.to_string_lossy()).unwrap();
    let loaded = RunnerConfig::load(&path.to_string_lossy()).unwrap();

    let _ = fs::remove_file(&path);

    assert_eq!(loaded.tasks.len(), 2);

    let crm_task = loaded.tasks.first().expect("No task");
    assert_eq!(crm_task.id, "crm_task");
    assert!(!crm_task.schedules.is_empty());
    match crm_task.schedules.first().expect("No schedules") {
        TaskSchedule::Interval { every_seconds, .. } => {
            assert_eq!(*every_seconds, 86400);
        }
        _ => panic!("Expected Interval schedule"),
    }

    let shell_task = &loaded.tasks[1];
    assert_eq!(shell_task.id, "shell_task");
    assert!(!shell_task.enabled);
    assert_eq!(shell_task.steps.len(), 1);

    if let ActionSpec::ShellCommand(cmd) = &shell_task.steps[0].actions[0] {
        assert_eq!(cmd.command, "echo Hello World");
    } else {
        panic!("Expected ShellCommand");
    }
}
