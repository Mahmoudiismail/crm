use anyhow::Result;
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::runner::config::{Repetition, ReportType, RunnerConfig, RunnerTask, TaskKind};
use crate::runner::engine::{create_task, delete_task, update_task};
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
    let (route_path, query_string) = split_path_and_query(path);
    let query = parse_query_string(query_string);

    if method == "GET" && route_path == "/" {
        let cfg = RunnerConfig::load(&handle.runner_config_path)?;
        let status = handle.status.lock().await.clone();

        let mut rows = String::new();
        for task in cfg.tasks {
            let repetition_label = match task.repetition {
                crate::runner::config::Repetition::Once => "once",
                crate::runner::config::Repetition::Repeat => "repeat",
            };
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><a href='/run/{}'>Run</a> | <a href='/enable/{}'>Enable</a> | <a href='/disable/{}'>Disable</a> | <a href='/edit/{}'>Edit</a> | <a href='/delete/{}'>Delete</a></td></tr>",
                escape_html(&task.id),
                escape_html(&task.name),
                task.enabled,
                repetition_label,
                task.frequency_seconds,
                escape_html(&task.next_run_at),
                escape_html(&task.last_status),
                escape_html(&task.id),
                escape_html(&task.id),
                escape_html(&task.id),
                escape_html(&task.id),
                escape_html(&task.id)
            ));
        }

        let html = format!(
            "<!doctype html><html><head><meta charset='utf-8'><title>Runner GUI</title></head><body><h1>Runner GUI</h1><p>Running: {}</p><p>Last task: {}</p><p>Last run at: {}</p><p>Last error: {}</p><p><a href='/run-all'>Run All Now</a> | <a href='/run-tickets'>Run CRM Tickets</a> | <a href='/new-task'>New Task</a> | <a href='/status'>JSON Status</a> | <a href='/tasks'>JSON Tasks</a></p><table border='1' cellpadding='6'><tr><th>ID</th><th>Name</th><th>Enabled</th><th>Repetition</th><th>Frequency(s)</th><th>Next Run</th><th>Last Status</th><th>Action</th></tr>{}</table></body></html>",
            status.currently_running,
            escape_html(&status.last_task_id),
            escape_html(&status.last_run_at),
            escape_html(&status.last_error),
            rows
        );
        return Ok((200, "text/html; charset=utf-8", html));
    }

    if method == "GET" && route_path == "/status" {
        let status = handle.status.lock().await.clone();
        let body = serde_json::to_string_pretty(&status)?;
        return Ok((200, "application/json", body));
    }

    if method == "GET" && route_path == "/tasks" {
        let cfg = RunnerConfig::load(&handle.runner_config_path)?;
        let body = serde_json::to_string_pretty(&cfg.tasks)?;
        return Ok((200, "application/json", body));
    }

    if method == "GET" && route_path == "/new-task" {
        let html = render_task_form("Create Task", "/create", "Create", None);
        return Ok((200, "text/html; charset=utf-8", html));
    }

    if method == "GET" && route_path.starts_with("/edit/") {
        let task_id = route_path.trim_start_matches("/edit/");
        let cfg = RunnerConfig::load(&handle.runner_config_path)?;
        if let Some(task) = cfg.tasks.iter().find(|t| t.id == task_id) {
            let action = format!("/update/{}", escape_html(task_id));
            let html = render_task_form("Edit Task", &action, "Update", Some(task));
            return Ok((200, "text/html; charset=utf-8", html));
        }
        return Ok((404, "text/plain", format!("Task '{}' not found", task_id)));
    }

    if method == "GET" && route_path == "/create" {
        let task = build_task_from_query(&query, None)?;
        create_task(&handle.runner_config_path, task).await?;
        return Ok((200, "text/plain", "Task created. Open / to view.".to_string()));
    }

    if method == "GET" && route_path.starts_with("/update/") {
        let task_id = route_path.trim_start_matches("/update/").to_string();
        let task = build_task_from_query(&query, Some(task_id.clone()))?;
        update_task(&handle.runner_config_path, &task_id, task).await?;
        return Ok((200, "text/plain", format!("Task '{}' updated. Open / to view.", task_id)));
    }

    if method == "GET" && route_path.starts_with("/delete/") {
        let task_id = route_path.trim_start_matches("/delete/").to_string();
        delete_task(&handle.runner_config_path, &task_id).await?;
        return Ok((200, "text/plain", format!("Task '{}' deleted. Open / to view.", task_id)));
    }

    if method == "GET" && route_path == "/run-all" {
        handle.command_tx.send(RunnerCommand::RunAllNow).await?;
        return Ok((200, "text/plain", "Triggered run-all".to_string()));
    }

    if method == "GET" && route_path == "/run-tickets" {
        handle
            .command_tx
            .send(RunnerCommand::RunAdhocCrm(ReportType::Tickets))
            .await?;
        return Ok((200, "text/plain", "Triggered CRM tickets run".to_string()));
    }

    if method == "GET" && route_path.starts_with("/run/") {
        let task_id = route_path.trim_start_matches("/run/").to_string();
        handle
            .command_tx
            .send(RunnerCommand::RunTaskNow(task_id.clone()))
            .await?;
        return Ok((200, "text/plain", format!("Triggered task {}", task_id)));
    }

    if method == "GET" && route_path.starts_with("/enable/") {
        let task_id = route_path.trim_start_matches("/enable/").to_string();
        handle
            .command_tx
            .send(RunnerCommand::SetTaskEnabled {
                task_id: task_id.clone(),
                enabled: true,
            })
            .await?;
        return Ok((200, "text/plain", format!("Enabled task {}", task_id)));
    }

    if method == "GET" && route_path.starts_with("/disable/") {
        let task_id = route_path.trim_start_matches("/disable/").to_string();
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

fn split_path_and_query(path: &str) -> (&str, &str) {
    match path.split_once('?') {
        Some((route, query)) => (route, query),
        None => (path, ""),
    }
}

fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = match pair.split_once('=') {
            Some((key, value)) => (key, value),
            None => (pair, ""),
        };
        values.insert(url_decode(raw_key), url_decode(raw_value));
    }
    values
}

fn url_decode(value: &str) -> String {
    let mut decoded = String::new();
    let bytes = value.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = &value[index + 1..index + 3];
                if let Ok(code) = u8::from_str_radix(hex, 16) {
                    decoded.push(code as char);
                    index += 3;
                } else {
                    decoded.push('%');
                    index += 1;
                }
            }
            byte => {
                decoded.push(byte as char);
                index += 1;
            }
        }
    }

    decoded
}

fn parse_checkbox(values: &HashMap<String, String>, key: &str) -> bool {
    values
        .get(key)
        .map(|v| {
            matches!(
                v.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn parse_report_type(raw: Option<&String>) -> ReportType {
    match raw.map(|v| v.to_ascii_lowercase()) {
        Some(v) if v == "tickets" => ReportType::Tickets,
        Some(v) if v == "calls" => ReportType::Calls,
        Some(v) if v == "leads" => ReportType::Leads,
        Some(v) if v == "none" => ReportType::None,
        _ => ReportType::All,
    }
}

fn build_task_from_query(values: &HashMap<String, String>, fallback_id: Option<String>) -> Result<RunnerTask> {
    let id = values
        .get("id")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or(fallback_id)
        .unwrap_or_default();

    let name = values
        .get("name")
        .map(|v| v.trim().to_string())
        .unwrap_or_default();

    let repetition = match values.get("repetition").map(|v| v.to_ascii_lowercase()) {
        Some(v) if v == "repeat" => Repetition::Repeat,
        _ => Repetition::Once,
    };

    let frequency_seconds = values
        .get("frequency_seconds")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(3600);

    let next_run_at = values
        .get("next_run_at")
        .map(|v| v.trim().to_string())
        .unwrap_or_default();

    let task_type = values
        .get("task_type")
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "crm_fetch".to_string());

    let kind = if task_type == "shell_command" {
        TaskKind::ShellCommand {
            command: values.get("command").cloned().unwrap_or_default(),
        }
    } else {
        TaskKind::CrmFetch {
            report: parse_report_type(values.get("report")),
        }
    };

    Ok(RunnerTask {
        id,
        name,
        enabled: parse_checkbox(values, "enabled"),
        repetition,
        frequency_seconds,
        next_run_at,
        kind,
        last_run_at: String::new(),
        last_status: String::new(),
    })
}

fn render_task_form(title: &str, action: &str, submit_label: &str, task: Option<&RunnerTask>) -> String {
    let id = task.map(|t| t.id.as_str()).unwrap_or_default();
    let name = task.map(|t| t.name.as_str()).unwrap_or_default();
    let enabled = task.map(|t| t.enabled).unwrap_or(true);
    let repetition = task.map(|t| &t.repetition).unwrap_or(&Repetition::Once);
    let frequency_seconds = task
        .map(|t| t.frequency_seconds.to_string())
        .unwrap_or_else(|| "3600".to_string());
    let next_run_at = task.map(|t| t.next_run_at.as_str()).unwrap_or_default();

    let (task_type, report, command) = match task.map(|t| &t.kind) {
        Some(TaskKind::ShellCommand { command }) => ("shell_command", "all", command.as_str()),
        Some(TaskKind::CrmFetch { report }) => (
            "crm_fetch",
            match report {
                ReportType::All => "all",
                ReportType::Tickets => "tickets",
                ReportType::Calls => "calls",
                ReportType::Leads => "leads",
                ReportType::None => "none",
            },
            "",
        ),
        None => ("crm_fetch", "all", ""),
    };

    format!(
        "<!doctype html><html><head><meta charset='utf-8'><title>{}</title></head><body><h1>{}</h1><p><a href='/'>Back</a></p><form method='get' action='{}'><p><label>ID: <input type='text' name='id' value='{}'></label></p><p><label>Name: <input type='text' name='name' value='{}'></label></p><p><label>Enabled: <input type='checkbox' name='enabled' value='on' {}></label></p><p><label>Repetition: <select name='repetition'><option value='once' {}>once</option><option value='repeat' {}>repeat</option></select></label></p><p><label>Frequency Seconds: <input type='number' name='frequency_seconds' min='0' value='{}'></label></p><p><label>Next Run At (RFC3339): <input type='text' name='next_run_at' value='{}'></label></p><p><label>Task Type: <select name='task_type'><option value='crm_fetch' {}>crm_fetch</option><option value='shell_command' {}>shell_command</option></select></label></p><p><label>CRM Report: <select name='report'><option value='all' {}>all</option><option value='tickets' {}>tickets</option><option value='calls' {}>calls</option><option value='leads' {}>leads</option><option value='none' {}>none</option></select></label></p><p><label>Shell Command: <input type='text' name='command' value='{}'></label></p><p><button type='submit'>{}</button></p></form></body></html>",
        escape_html(title),
        escape_html(title),
        action,
        escape_html(id),
        escape_html(name),
        if enabled { "checked" } else { "" },
        if matches!(repetition, Repetition::Once) { "selected" } else { "" },
        if matches!(repetition, Repetition::Repeat) { "selected" } else { "" },
        escape_html(&frequency_seconds),
        escape_html(next_run_at),
        if task_type == "crm_fetch" { "selected" } else { "" },
        if task_type == "shell_command" { "selected" } else { "" },
        if report == "all" { "selected" } else { "" },
        if report == "tickets" { "selected" } else { "" },
        if report == "calls" { "selected" } else { "" },
        if report == "leads" { "selected" } else { "" },
        if report == "none" { "selected" } else { "" },
        escape_html(command),
        escape_html(submit_label)
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
