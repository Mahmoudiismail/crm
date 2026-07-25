use anyhow::Result;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::runner::config::RunnerConfig;
use crate::runner::engine::RunnerHandle;

pub mod api;
pub mod assets;
pub mod forms;
pub mod handlers;
pub mod helpers;
pub mod response;
pub mod routes;
pub mod templates;
pub mod validation;

use response::render_error_page;
use routes::{read_http_request, route_request};

pub fn start_gui_server(handle: RunnerHandle) {
    tokio::spawn(async move {
        if let Err(e) = run_server(handle).await {
            error!("Runner GUI server failed: {:#}", e);
        }
    });
}

async fn run_server(handle: RunnerHandle) -> Result<()> {
    let cfg = RunnerConfig::load(&handle.runner_config_path)?;
    let bind_addr = format!("{}:{}", cfg.gui_host, cfg.gui_port);
    let listener = TcpListener::bind(&bind_addr).await?;
    info!("Runner GUI listening on http://{}", bind_addr);

    loop {
        let (mut socket, _) = listener.accept().await?;
        let handle_clone = handle.clone();

        tokio::spawn(async move {
            let request = match read_http_request(&mut socket).await {
                Ok(Some(request)) => request,
                Ok(None) | Err(_) => return,
            };

            let (status, content_type, body) = match route_request(&request, &handle_clone).await {
                Ok(v) => v,
                Err(e) => (
                    500,
                    "text/html; charset=utf-8",
                    render_error_page("Request failed", &format!("{e}")),
                ),
            };

            let response = format!(
                "HTTP/1.1 {} OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status,
                content_type,
                body.len(),
                body
            );

            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::engine::RunnerStatus;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::{mpsc, Mutex};

    #[tokio::test]
    async fn test_start_gui_server_routing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_file = temp_dir.path().join("config.json");
        let config_path = config_file.to_str().unwrap().to_string();

        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();

        let cfg = RunnerConfig {
            gui_port: port,
            ..Default::default()
        };
        cfg.save(&config_path).unwrap();

        let (tx, _rx) = mpsc::channel(1);
        let status = Arc::new(Mutex::new(RunnerStatus {
            running_tasks_count: 1,
            queued_tasks_count: 0,
            last_error: "Test Error".to_string(),
            last_task_id: "test_task".to_string(),
            last_run_at: "2024-01-01T00:00:00Z".to_string(),
        }));

        let (exec_tx, _) = mpsc::channel(128);
        let handle = RunnerHandle {
            command_tx: tx,
            exec_tx,
            status,
            runner_config_path: config_path.clone(),
        };

        // Start the server
        start_gui_server(handle);

        // Give it a moment to start and bind
        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = reqwest::Client::new();

        // Test GET /
        let res = client
            .get(format!("http://127.0.0.1:{}/", port))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        let text = res.text().await.unwrap();
        assert!(text.contains("Task Dashboard"));

        // Test GET /status
        let res = client
            .get(format!("http://127.0.0.1:{}/status", port))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        let status_json: serde_json::Value = res.json().await.unwrap();
        assert_eq!(status_json["last_task_id"], "test_task");
        assert_eq!(status_json["last_error"], "Test Error");
    }
}
