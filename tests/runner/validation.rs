use crm_tool::runner::config::{
    ActionSpec, ExecutionMode, ExternalAppSpec, RegisteredApp, Repetition, RunnerConfig,
    RunnerTask, ShellCommandSpec, TaskStep,
};
use crm_tool::runner::engine::errors::EngineError;
use crm_tool::runner::engine::validation::validate_config;

#[test]
fn test_validate_config_duplicate_task_id() {
    let mut config = RunnerConfig::default();
    let step = TaskStep {
        name: None,
        mode: ExecutionMode::Sequential,
        actions: vec![ActionSpec::ShellCommand(ShellCommandSpec {
            command: "echo 1".to_string(),
            continue_on_error: false,
        })],
    };
    let task = RunnerTask {
        id: "task_1".to_string(),
        name: "Task 1".to_string(),
        enabled: true,
        repetition: Repetition::Repeat,
        frequency_seconds: 60,
        next_run_at: "".to_string(),
        last_run_at: "".to_string(),
        last_status: "".to_string(),
        timeout_seconds: 0,
        schedules: vec![],
        steps: vec![step.clone()],
        post_run_steps: vec![],
    };
    config.tasks = vec![task.clone(), task.clone()];

    let result = validate_config(&config);
    assert!(result.is_err());
    if let Err(EngineError::Validation(msg)) = result {
        assert!(msg.contains("Duplicate task ID"));
    } else {
        panic!("Expected Validation error");
    }
}

#[test]
fn test_validate_config_duplicate_app_id() {
    let mut config = RunnerConfig::default();
    let app = RegisteredApp {
        id: "app_1".to_string(),
        name: "App 1".to_string(),
        executable_path: "app.exe".to_string(),
        config_path: "app_config.json".to_string(),
        plugin_api_version: None,

        allow_concurrent_tasks: false,
    };
    config.registered_apps = vec![app.clone(), app.clone()];

    let result = validate_config(&config);
    assert!(result.is_err());
    if let Err(EngineError::Validation(msg)) = result {
        assert!(msg.contains("Duplicate application ID"));
    } else {
        panic!("Expected Validation error");
    }
}

#[test]
fn test_validate_config_empty_steps() {
    let mut config = RunnerConfig::default();
    let task = RunnerTask {
        id: "task_1".to_string(),
        name: "Task 1".to_string(),
        enabled: true,
        repetition: Repetition::Repeat,
        frequency_seconds: 60,
        next_run_at: "".to_string(),
        last_run_at: "".to_string(),
        last_status: "".to_string(),
        timeout_seconds: 0,
        schedules: vec![],
        steps: vec![], // Empty steps
        post_run_steps: vec![],
    };
    config.tasks = vec![task];

    let result = validate_config(&config);
    assert!(result.is_err());
    if let Err(EngineError::Validation(msg)) = result {
        assert!(msg.contains("no steps in its pipeline"));
    } else {
        panic!("Expected Validation error");
    }
}

#[test]
fn test_validate_config_empty_action_list_in_step() {
    let mut config = RunnerConfig::default();
    let step = TaskStep {
        name: None,
        mode: ExecutionMode::Sequential,
        actions: vec![], // Empty actions list
    };
    let task = RunnerTask {
        id: "task_1".to_string(),
        name: "Task 1".to_string(),
        enabled: true,
        repetition: Repetition::Repeat,
        frequency_seconds: 60,
        next_run_at: "".to_string(),
        last_run_at: "".to_string(),
        last_status: "".to_string(),
        timeout_seconds: 0,
        schedules: vec![],
        steps: vec![step],
        post_run_steps: vec![],
    };
    config.tasks = vec![task];

    let result = validate_config(&config);
    assert!(result.is_err());
    if let Err(EngineError::Validation(msg)) = result {
        assert!(msg.contains("contains an empty step"));
    } else {
        panic!("Expected Validation error");
    }
}

#[test]
fn test_validate_config_invalid_external_app_reference() {
    let mut config = RunnerConfig::default();
    let step = TaskStep {
        name: None,
        mode: ExecutionMode::Sequential,
        actions: vec![ActionSpec::ExternalApp(ExternalAppSpec {
            app_id: "missing_app".to_string(),
            args: std::collections::HashMap::new(),
            period_mode: crm_tool::runner::config::PeriodMode::Custom,
            start_date: None,
            end_date: None,
            start_date_arg: None,
            end_date_arg: None,
        })],
    };
    let task = RunnerTask {
        id: "task_1".to_string(),
        name: "Task 1".to_string(),
        enabled: true,
        repetition: Repetition::Repeat,
        frequency_seconds: 60,
        next_run_at: "".to_string(),
        last_run_at: "".to_string(),
        last_status: "".to_string(),
        timeout_seconds: 0,
        schedules: vec![],
        steps: vec![step],
        post_run_steps: vec![],
    };
    config.tasks = vec![task];
    // Notice registered_apps is empty

    let result = validate_config(&config);
    assert!(result.is_err());
    if let Err(EngineError::Validation(msg)) = result {
        assert!(msg.contains("references unknown app_id"));
    } else {
        panic!("Expected Validation error");
    }
}

#[test]
fn legacy_registered_app_config_defaults_to_unversioned() {
    let app: RegisteredApp = serde_json::from_value(serde_json::json!({
        "id": "legacy",
        "name": "Legacy app",
        "executable_path": "legacy.exe",
        "config_path": "legacy.json",
        "allow_concurrent_tasks": false
    }))
    .unwrap();

    assert_eq!(app.plugin_api_version, None);
    let serialized = serde_json::to_value(app).unwrap();
    assert!(serialized.get("plugin_api_version").is_none());
}

#[test]
fn validate_config_rejects_unsupported_registered_plugin_api_version() {
    let mut config = RunnerConfig::default();
    config.registered_apps.push(RegisteredApp {
        id: "future_plugin".to_string(),
        name: "Future plugin".to_string(),
        executable_path: "future.exe".to_string(),
        config_path: String::new(),
        plugin_api_version: Some(2),
        allow_concurrent_tasks: false,
    });

    let error = validate_config(&config).unwrap_err();
    assert!(
        matches!(error, EngineError::Validation(message) if message.contains("Runner supports 1"))
    );
}
