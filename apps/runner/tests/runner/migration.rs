use crm_tool::runner::config::{ActionSpec, ExecutionMode, RunnerTask};

#[test]
fn test_legacy_configuration_migration() {
    let legacy_json = r#"{
        "id": "legacy-task-1",
        "name": "Legacy Task",
        "enabled": true,
        "kind": {
            "type": "shell_command",
            "mode": "sequential",
            "commands": [
                {
                    "command": "echo Hello",
                    "continue_on_error": false
                }
            ]
        },
        "post_run_script": "echo Post",
        "post_run_app_id": "app1",
        "post_run_app_args": {
            "arg1": "val1"
        }
    }"#;

    let task: RunnerTask = serde_json::from_str(legacy_json).unwrap();
    assert_eq!(task.id, "legacy-task-1");
    assert_eq!(task.name, "Legacy Task");

    // Assert Canonical Translation
    assert_eq!(task.steps.len(), 1);
    assert_eq!(task.steps[0].mode, ExecutionMode::Sequential);
    assert_eq!(task.steps[0].actions.len(), 1);
    if let ActionSpec::ShellCommand(c) = &task.steps[0].actions[0] {
        assert_eq!(c.command, "echo Hello");
    } else {
        panic!("Expected ShellCommand");
    }

    assert_eq!(task.post_run_steps.len(), 1);
    assert_eq!(task.post_run_steps[0].actions.len(), 2);
}
