#![allow(unused_imports)]
use crate::runner::config::*;
use crate::runner::engine::*;
use anyhow::{Context, Result};
use chrono::{Local, Utc};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info};

pub mod components;
pub mod forms;
pub mod handlers;
pub mod helpers;
pub mod icons;
pub mod routes;
pub mod templates;

use forms::*;
use helpers::*;
use routes::route_request;
use templates::render_error_page;

pub(crate) const TAILWIND_CDN: &str = "https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4";

pub(crate) struct HttpRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) body: String,
}

pub fn start_gui_server(handle: RunnerHandle) {
    tokio::spawn(async move {
        if let Err(e) = run_server(handle).await {
            error!("Runner GUI server failed: {:#}", e);
        }
    });
}

pub(crate) async fn run_server(handle: RunnerHandle) -> Result<()> {
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
                Ok(v) => {
                    if v.0 == 404 && !request.path.starts_with("/assets/js/") {
                        tracing::warn!("HTTP 404 Not Found: {}", request.path);
                    }
                    v
                }
                Err(e) => {
                    tracing::error!("HTTP 500 Internal Error on {}: {:#}", request.path, e);
                    (
                        500,
                        "text/html; charset=utf-8",
                        render_error_page("Request failed", &format!("{e}")),
                    )
                }
            };

            let cache_header = if request.path.starts_with("/assets/js/") {
                "Cache-Control: public, max-age=604800\r\n"
            } else {
                "Cache-Control: no-cache\r\n"
            };
            let response = format!(
                "HTTP/1.1 {} OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n{}",
                status,
                content_type,
                body.len(),
                cache_header,
                body
            );

            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        });
    }
}

pub(crate) async fn read_http_request(
    socket: &mut tokio::net::TcpStream,
) -> Result<Option<HttpRequest>> {
    let mut buf = vec![0u8; 8192];
    let mut read = socket.read(&mut buf).await?;
    if read == 0 {
        return Ok(None);
    }

    let mut content_length = header_content_length(&buf[..read]).unwrap_or(0);
    while body_len(&buf[..read]) < content_length {
        if read == buf.len() {
            buf.resize(buf.len() * 2, 0);
        }
        let n = socket.read(&mut buf[read..]).await?;
        if n == 0 {
            break;
        }
        read += n;
        content_length = header_content_length(&buf[..read]).unwrap_or(content_length);
    }

    let req = String::from_utf8_lossy(&buf[..read]);
    let (headers, body) = req.split_once("\r\n\r\n").unwrap_or((req.as_ref(), ""));
    let first = headers.lines().next().unwrap_or_default();
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or("/").to_string();

    if !path.starts_with("/assets/js/") {
        info!(
            "HTTP Request: {} {}\nHeaders:\n{}\nBody:\n{}",
            method, path, headers, body
        );
    }

    Ok(Some(HttpRequest {
        method,
        path,
        body: body.to_string(),
    }))
}

pub(crate) fn header_content_length(bytes: &[u8]) -> Option<usize> {
    let req = String::from_utf8_lossy(bytes);
    req.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("content-length") {
            value.trim().parse().ok()
        } else {
            None
        }
    })
}

pub(crate) fn body_len(bytes: &[u8]) -> usize {
    bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|idx| bytes.len().saturating_sub(idx + 4))
        .unwrap_or(0)
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
            running_task_ids: Vec::new(),
            queued_task_ids: Vec::new(),
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
        assert!(text.contains("Runner"));

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

        // Test GET /api/apps/list (ensure apps endpoint doesn't return 404 or 500)
        let res = client
            .get(format!("http://127.0.0.1:{}/api/apps/list", port))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        let text = res.text().await.unwrap();
        assert!(text.starts_with('['));

        // Test POST /tasks/create mimicking forms.js output
        let payload = [
            ("id", "report_callcenter"),
            ("name", "report callcenter"),
            ("enabled", "on"),
            ("timeout_seconds", "9999"),
            ("schedules", "interval: every 1h; st: 07:00; wh: Saturday=07:00-23:00,Sunday=07:00-23:00,Monday=07:00-23:00,Tuesday=07:00-23:00,Wednesday=07:00-23:00,Thursday=07:00-23:00,Friday=07:00-23:00\ndaily: 12:00, 15:00; wh: Monday=09:00-17:00\nweekly: Monday; st: 14:00\nmonthly: day 15; st: 10:30"),
            ("steps", "[{\"name\":\"Legacy External App\",\"mode\":\"sequential\",\"actions\":[{\"type\":\"external_app\",\"app_id\":\"CRM\",\"args\":{\"--report\":\"tickets,leads\"}}]}]"),
            ("post_run_steps", "[]")
        ];

        let mut form_encoded = String::new();
        for (k, v) in payload.iter() {
            if !form_encoded.is_empty() {
                form_encoded.push('&');
            }
            form_encoded.push_str(&urlencoding::encode(k));
            form_encoded.push('=');
            form_encoded.push_str(&urlencoding::encode(v));
        }

        let res = client
            .post(format!("http://127.0.0.1:{}/create", port))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_encoded)
            .send()
            .await
            .unwrap();

        // Should redirect to dashboard on success
        assert_eq!(res.status().as_u16(), 200);

        let saved_cfg = RunnerConfig::load(&config_path).unwrap();
        let saved_task = saved_cfg
            .tasks
            .iter()
            .find(|t| t.id == "report_callcenter")
            .unwrap();
        assert_eq!(saved_task.name, "report callcenter");
        assert!(saved_task.enabled);
        assert_eq!(saved_task.schedules.len(), 4);

        match &saved_task.schedules[0] {
            TaskSchedule::Interval {
                every_seconds,
                working_hours,
                start_time,
                ..
            } => {
                assert_eq!(*every_seconds, 3600);
                assert_eq!(start_time.as_deref(), Some("07:00"));
                let wh = working_hours.as_ref().unwrap();
                assert_eq!(wh.len(), 7);
                assert_eq!(wh.get("Saturday").unwrap().start, "07:00");
                assert_eq!(wh.get("Saturday").unwrap().end, "23:00");
            }
            _ => panic!("Expected interval schedule"),
        }
    }

    #[test]
    fn parses_schedule_text() {
        let schedules = parse_schedules_text(
            "interval: every 1h\ndaily: 09:00, 13:00\nonce: 2026-04-15T09:30:00-05:00",
            &std::collections::HashMap::new(),
            &[],
        )
        .unwrap();
        assert_eq!(schedules.len(), 3);
        match schedules.first().expect("No schedule") {
            TaskSchedule::Interval {
                every_seconds,
                working_hours,
                ..
            } => {
                assert_eq!(*every_seconds, 3_600);
                assert!(working_hours.is_none());
            }
            _ => panic!("expected interval"),
        }
    }

    #[test]
    fn parses_schedule_text_with_working_hours() {
        let schedules = parse_schedules_text(
            "interval: every 2h; wh: Monday=09:00-17:00,Friday=10:00-15:00\n",
            &std::collections::HashMap::new(),
            &[],
        )
        .unwrap();
        assert_eq!(schedules.len(), 1);
        match schedules.first().expect("No schedule") {
            TaskSchedule::Interval {
                every_seconds,
                working_hours,
                ..
            } => {
                assert_eq!(*every_seconds, 7_200);
                let wh = working_hours.as_ref().unwrap();
                assert_eq!(wh.len(), 2);
                assert_eq!(wh.get("Monday").unwrap().start, "09:00");
                assert_eq!(wh.get("Monday").unwrap().end, "17:00");
                assert_eq!(wh.get("Friday").unwrap().start, "10:00");
                assert_eq!(wh.get("Friday").unwrap().end, "15:00");
            }
            _ => panic!("expected interval"),
        }
    }

    #[test]
    fn parses_schedule_text_weekly_monthly_with_start_time() {
        let schedules = parse_schedules_text(
            "weekly: Monday; st: 14:00\nmonthly: day 15; st: 10:30",
            &std::collections::HashMap::new(),
            &[],
        )
        .unwrap();
        assert_eq!(schedules.len(), 2);
        match &schedules[0] {
            TaskSchedule::Weekly { at_time, .. } => assert_eq!(at_time, "14:00"),
            _ => panic!("expected weekly"),
        }
        match &schedules[1] {
            TaskSchedule::Monthly { at_time, .. } => assert_eq!(at_time, "10:30"),
            _ => panic!("expected monthly"),
        }
    }

    #[test]
    fn duration_parser_accepts_human_units() {
        assert_eq!(parse_duration_text("1h").unwrap(), 3_600);
        assert_eq!(parse_duration_text("1h 30m").unwrap(), 5_400);
        assert_eq!(parse_duration_text("90").unwrap(), 90);
    }

    #[test]
    fn human_datetime_accepts_rfc3339() {
        let text = human_datetime(&Utc::now().to_rfc3339());
        assert!(text.contains("local"));
    }

    #[test]
    fn date_type_import_keeps_rfc3339_parse_available() {
        let parsed: chrono::DateTime<Utc> = parse_rfc3339_utc("2026-04-15T09:30:00Z").unwrap();
        assert_eq!(parsed.to_rfc3339(), "2026-04-15T09:30:00+00:00");
    }
}
