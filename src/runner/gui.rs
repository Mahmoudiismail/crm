use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::runner::config::RunnerConfig;
use crate::runner::engine::{RunnerCommand, RunnerHandle};

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
            let mut buf = vec![0u8; 8192];
            let read = match socket.read(&mut buf).await {
                Ok(n) => n,
                Err(_) => return,
            };
            if read == 0 {
                return;
            }

            let req = String::from_utf8_lossy(&buf[..read]);
            let mut lines = req.lines();
            let first = lines.next().unwrap_or_default();
            let mut parts = first.split_whitespace();
            let method = parts.next().unwrap_or_default();
            let path = parts.next().unwrap_or("/");

            let (status, content_type, body) = match route_request(method, path, &handle_clone).await {
                Ok(v) => v,
                Err(e) => (500, "text/plain", format!("error: {}", e)),
            };

            let response = format!(
                "HTTP/1.1 {} OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status,
                content_type,
                body.as_bytes().len(),
                body
            );

            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        });
    }
}

async fn route_request(method: &str, path: &str, handle: &RunnerHandle) -> Result<(u16, &'static str, String)> {
    if method == "GET" && path == "/" {
        let cfg = RunnerConfig::load(&handle.runner_config_path)?;
        let status = handle.status.lock().await.clone();

        let mut rows = String::new();
        for task in cfg.tasks {
            let repetition_label = match task.repetition {
                crate::runner::config::Repetition::Once => "once",
                crate::runner::config::Repetition::Repeat => "repeat",
            };
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><a href='/run/{}'>Run</a> | <a href='/enable/{}'>Enable</a> | <a href='/disable/{}'>Disable</a></td></tr>",
                task.id,
                task.name,
                task.enabled,
                repetition_label,
                task.frequency_seconds,
                task.next_run_at,
                task.last_status,
                task.id,
                task.id,
                task.id
            ));
        }

        let html = format!(
            "<!doctype html><html><head><meta charset='utf-8'><title>Runner GUI</title></head><body><h1>Runner GUI</h1><p>Running: {}</p><p>Last task: {}</p><p>Last run at: {}</p><p>Last error: {}</p><p><a href='/run-all'>Run All Now</a> | <a href='/run-tickets'>Run CRM Tickets</a> | <a href='/status'>JSON Status</a> | <a href='/tasks'>JSON Tasks</a></p><table border='1' cellpadding='6'><tr><th>ID</th><th>Name</th><th>Enabled</th><th>Repetition</th><th>Frequency(s)</th><th>Next Run</th><th>Last Status</th><th>Action</th></tr>{}</table></body></html>",
            status.currently_running,
            status.last_task_id,
            status.last_run_at,
            status.last_error,
            rows
        );
        return Ok((200, "text/html; charset=utf-8", html));
    }

    if method == "GET" && path == "/status" {
        let status = handle.status.lock().await.clone();
        let body = serde_json::to_string_pretty(&status)?;
        return Ok((200, "application/json", body));
    }

    if method == "GET" && path == "/tasks" {
        let cfg = RunnerConfig::load(&handle.runner_config_path)?;
        let body = serde_json::to_string_pretty(&cfg.tasks)?;
        return Ok((200, "application/json", body));
    }

    if method == "GET" && path == "/run-all" {
        handle.command_tx.send(RunnerCommand::RunAllNow).await?;
        return Ok((200, "text/plain", "Triggered run-all".to_string()));
    }

    if method == "GET" && path == "/run-tickets" {
        handle
            .command_tx
            .send(RunnerCommand::RunAdhocCrm(crate::crm::types::ReportType::Tickets))
            .await?;
        return Ok((200, "text/plain", "Triggered CRM tickets run".to_string()));
    }

    if method == "GET" && path.starts_with("/run/") {
        let task_id = path.trim_start_matches("/run/").to_string();
        handle
            .command_tx
            .send(RunnerCommand::RunTaskNow(task_id.clone()))
            .await?;
        return Ok((200, "text/plain", format!("Triggered task {}", task_id)));
    }

    if method == "GET" && path.starts_with("/enable/") {
        let task_id = path.trim_start_matches("/enable/").to_string();
        handle
            .command_tx
            .send(RunnerCommand::SetTaskEnabled {
                task_id: task_id.clone(),
                enabled: true,
            })
            .await?;
        return Ok((200, "text/plain", format!("Enabled task {}", task_id)));
    }

    if method == "GET" && path.starts_with("/disable/") {
        let task_id = path.trim_start_matches("/disable/").to_string();
        handle
            .command_tx
            .send(RunnerCommand::SetTaskEnabled {
                task_id: task_id.clone(),
                enabled: false,
            })
            .await?;
        return Ok((200, "text/plain", format!("Disabled task {}", task_id)));
    }

    Ok((404, "text/plain", "Not found".to_string()))
}
