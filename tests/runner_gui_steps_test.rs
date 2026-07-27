// Added as required by test mandate for bugfixes
#[cfg(test)]
mod tests {
    use crm_tool::runner::config::{ExecutionMode, TaskStep};
    use std::collections::HashMap;

    #[test]
    fn test_parse_multiple_steps() {
        let mut values = HashMap::new();
        values.insert("id".to_string(), "test_task".to_string());
        values.insert("name".to_string(), "Test Task".to_string());

        let steps_json = r#"[
            {"name": "Step 1", "mode": "sequential", "actions": [{"type": "shell_command", "command": "echo 1", "continue_on_error": false}]},
            {"name": "Step 2", "mode": "parallel", "actions": [{"type": "shell_command", "command": "echo 2", "continue_on_error": true}]}
        ]"#;
        values.insert("steps".to_string(), steps_json.to_string());

        let post_run_steps_json = r#"[
            {"name": "Post 1", "mode": "sequential", "actions": []}
        ]"#;
        values.insert(
            "post_run_steps".to_string(),
            post_run_steps_json.to_string(),
        );

        // This tests the `build_task_from_values` indirectly or simulates its parsing logic
        let steps: Vec<TaskStep> = serde_json::from_str(steps_json).unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].name.as_deref(), Some("Step 1"));
        assert!(matches!(steps[0].mode, ExecutionMode::Sequential));
        assert!(matches!(steps[1].mode, ExecutionMode::Parallel));

        let post_run_steps: Vec<TaskStep> = serde_json::from_str(post_run_steps_json).unwrap();
        assert_eq!(post_run_steps.len(), 1);
    }
}
