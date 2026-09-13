use crm_tool::runner::config::RunnerConfig;
use crm_tool::runner::gui::routes::route_request;
use crm_tool::runner::gui::HttpRequest;

#[test]
fn test_non_loopback_gui_host_rejected() {
    let cfg1 = RunnerConfig {
        gui_host: "127.0.0.1".to_string(),
        ..Default::default()
    };
    assert!(cfg1.validate().is_ok());

    let cfg2 = RunnerConfig {
        gui_host: "localhost".to_string(),
        ..Default::default()
    };
    assert!(cfg2.validate().is_ok());

    let cfg3 = RunnerConfig {
        gui_host: "::1".to_string(),
        ..Default::default()
    };
    assert!(cfg3.validate().is_ok());

    let cfg4 = RunnerConfig {
        gui_host: "[::1]".to_string(),
        ..Default::default()
    };
    assert!(cfg4.validate().is_ok());

    // Non-loopback hosts must fail validation with security error
    let cfg_invalid1 = RunnerConfig {
        gui_host: "0.0.0.0".to_string(),
        ..Default::default()
    };
    let err = cfg_invalid1.validate().unwrap_err();
    assert!(
        err.to_string()
            .contains("Security error: Binding to non-loopback gui_host '0.0.0.0'"),
        "Error message should mention security error: {}",
        err
    );

    let cfg_invalid2 = RunnerConfig {
        gui_host: "192.168.1.50".to_string(),
        ..Default::default()
    };
    let err2 = cfg_invalid2.validate().unwrap_err();
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

#[tokio::test]
async fn test_http_security_and_limits() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir
        .path()
        .join("config.json")
        .to_str()
        .unwrap()
        .to_string();

    let cfg = RunnerConfig {
        gui_port: port,
        gui_host: "127.0.0.1".to_string(),
        ..Default::default()
    };
    cfg.save(&config_path).unwrap();

    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    let (exec_tx, _) = tokio::sync::mpsc::channel(128);
    let status = std::sync::Arc::new(tokio::sync::Mutex::new(
        crm_tool::runner::engine::RunnerStatus {
            running_tasks_count: 0,
            queued_tasks_count: 0,
            running_task_ids: Vec::new(),
            queued_task_ids: Vec::new(),
            last_error: String::new(),
            last_task_id: String::new(),
            last_run_at: String::new(),
            waiting_for_app: std::collections::HashMap::new(),
        },
    ));

    let handle = crm_tool::runner::engine::RunnerHandle {
        command_tx: tx,
        exec_tx,
        status,
        runner_config_path: config_path,
    };

    crm_tool::runner::gui::start_gui_server(handle);
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // 1. Chunked Transfer-Encoding rejection
    {
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .unwrap();
        let req = "POST /api/tasks/preview HTTP/1.1
Host: 127.0.0.1
Transfer-Encoding: chunked

5
hello
0

";
        stream.write_all(req.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();

        let mut resp = String::new();
        stream.read_to_string(&mut resp).await.unwrap();
        assert!(
            resp.contains("HTTP/1.1 400 Bad Request"),
            "Expected 400 Bad Request, got: {}",
            resp
        );
    }

    // 2. Content-Length > 2MB limit (413 Payload Too Large)
    {
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .unwrap();
        let req = "POST /create HTTP/1.1
Host: 127.0.0.1
Content-Length: 3000000

";
        stream.write_all(req.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();

        let mut resp = String::new();
        stream.read_to_string(&mut resp).await.unwrap();
        assert!(
            resp.contains("HTTP/1.1 413 Payload Too Large"),
            "Expected 413 Payload Too Large, got: {}",
            resp
        );
    }

    // 3. Status reason phrase formatting for 404
    {
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .unwrap();
        let req = "GET /nonexistent_route_test HTTP/1.1
Host: 127.0.0.1

";
        stream.write_all(req.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();

        let mut resp = String::new();
        stream.read_to_string(&mut resp).await.unwrap();
        assert!(
            resp.contains("HTTP/1.1 404 Not Found"),
            "Expected 404 Not Found, got: {}",
            resp
        );
    }
}
