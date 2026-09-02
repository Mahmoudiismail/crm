import sys

with open('tests/runner/loader.rs', 'r') as f:
    content = f.read()

new_test = """
#[test]
fn test_empty_schedule_is_manual_persistence() {
    let task = RunnerTask {
        id: "manual_task".to_string(),
        name: "Manual Only".to_string(),
        enabled: true,
        repetition: Repetition::Once,
        frequency_seconds: 0,
        next_run_at: String::new(),
        schedules: vec![],
        steps: Vec::new(),
        post_run_steps: Vec::new(),
        last_run_at: String::new(),
        last_status: String::new(),
        timeout_seconds: 0,
    };

    let legacy_val = serde_json::to_value(crm_tool::runner::config::migration::RunnerTaskLegacy::from(task.clone())).unwrap();
    let loaded: RunnerTask = serde_json::from_value::<crm_tool::runner::config::migration::RunnerTaskLegacy>(legacy_val).unwrap().into();

    assert!(loaded.schedules.is_empty(), "Expected empty schedules to persist as empty (Manual)");
}
"""

content = content.replace("fn test_mixed_tasks_persistence() {", new_test + "\n#[test]\nfn test_mixed_tasks_persistence() {")

with open('tests/runner/loader.rs', 'w') as f:
    f.write(content)
