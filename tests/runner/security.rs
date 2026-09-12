use crm_tool::runner::config::RunnerConfig;
use crm_tool::runner::gui::routes::route_request;
use crm_tool::runner::gui::HttpRequest;

#[test]
fn test_non_loopback_gui_host_rejected() {
    let mut cfg = RunnerConfig::default();

    // Loopback hosts must succeed
    cfg.gui_host = "127.0.0.1".to_string();
    assert!(cfg.validate().is_ok());

    cfg.gui_host = "localhost".to_string();
    assert!(cfg.validate().is_ok());

    cfg.gui_host = "::1".to_string();
    assert!(cfg.validate().is_ok());

    cfg.gui_host = "[::1]".to_string();
    assert!(cfg.validate().is_ok());

    // Non-loopback hosts must fail validation with security error
    cfg.gui_host = "0.0.0.0".to_string();
    let err = cfg.validate().unwrap_err();
    assert!(
        err.to_string()
            .contains("Security error: Binding to non-loopback gui_host '0.0.0.0'"),
        "Error message should mention security error: {}",
        err
    );

    cfg.gui_host = "192.168.1.50".to_string();
    let err2 = cfg.validate().unwrap_err();
    assert!(err2
        .to_string()
        .contains("Security error: Binding to non-loopback gui_host"));
}

#[tokio::test]
async fn test_get_mutation_rejected_405() {
    use crm_tool::runner::engine::RunnerHandle;
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use tokio::sync::Mutex;

    let (tx1, _rx1) = mpsc::channel(10);
    let (tx2, _rx2) = mpsc::channel(10);
    let status = Arc::new(Mutex::new(crm_tool::runner::engine::RunnerStatus {
        running_tasks_count: 0,
        queued_tasks_count: 0,
        running_task_ids: vec![],
        queued_task_ids: vec![],
        last_error: String::new(),
        last_task_id: String::new(),
        last_run_at: String::new(),
        waiting_for_app: std::collections::HashMap::new(),
    }));

    let handle = RunnerHandle {
        command_tx: tx1,
        exec_tx: tx2,
        status,
        runner_config_path: "runner_config.json".to_string(),
    };

    let mutation_paths = vec![
        "/create",
        "/update/task1",
        "/delete/task1",
        "/run/task1",
        "/run-all",
        "/enable/task1",
        "/disable/task1",
        "/reload",
        "/working-hours/create",
        "/working-hours/update/wh1",
        "/working-hours/delete/wh1",
        "/apps/create",
        "/apps/update/app1",
        "/apps/delete/app1",
    ];

    for path in mutation_paths {
        let request = HttpRequest {
            method: "GET".to_string(),
            path: path.to_string(),
            body: String::new(),
        };

        let (status_code, _content_type, body) = route_request(&request, &handle).await.unwrap();
        assert_eq!(
            status_code, 405,
            "GET request to mutation route '{}' should return 405 Method Not Allowed, got {}",
            path, status_code
        );
        assert_eq!(body, "Method Not Allowed");
    }
}
