const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

pub fn status_reason_phrase(code: u16) -> &'static str {
    match code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        _ => "Unknown Error",
    }
}
#[allow(unused_imports)]
use crate::runner::config::*;
use crate::runner::engine::*;
use anyhow::Result;

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

use routes::route_request;
use templates::render_error_page;

pub(crate) const TAILWIND_CDN: &str = "https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4";

#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub body: String,
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
            let reason = status_reason_phrase(status);
            let response = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n{}",
                status,
                reason,
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
    use std::time::Duration;

    let mut buf = vec![0u8; 8192];
    let mut read = 0;

    loop {
        if read >= MAX_HEADER_BYTES + MAX_BODY_BYTES {
            let resp = "HTTP/1.1 413 Payload Too Large\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nPayload Too Large";
            let _ = socket.write_all(resp.as_bytes()).await;
            let _ = socket.shutdown().await;
            return Ok(None);
        }

        let read_res =
            tokio::time::timeout(Duration::from_secs(10), socket.read(&mut buf[read..])).await;
        let n = match read_res {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => n,
            Ok(Err(_)) => break,
            Err(_) => {
                let resp = "HTTP/1.1 408 Request Timeout\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nRequest Timeout";
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.shutdown().await;
                return Ok(None);
            }
        };

        read += n;

        let req_str = String::from_utf8_lossy(&buf[..read]);
        if req_str.lines().any(|l| {
            let mut parts = l.splitn(2, ':');
            if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
                k.trim().eq_ignore_ascii_case("transfer-encoding")
                    && v.to_lowercase().contains("chunked")
            } else {
                false
            }
        }) {
            let resp = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nChunked Transfer-Encoding is not supported";
            let _ = socket.write_all(resp.as_bytes()).await;
            let _ = socket.shutdown().await;
            return Ok(None);
        }

        let header_end = buf[..read]
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|p| (p, 4))
            .or_else(|| {
                buf[..read]
                    .windows(2)
                    .position(|w| w == b"\n\n")
                    .map(|p| (p, 2))
            });

        if let Some((pos, delim_len)) = header_end {
            if pos + delim_len > MAX_HEADER_BYTES {
                let resp = "HTTP/1.1 413 Payload Too Large\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nPayload Too Large";
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.shutdown().await;
                return Ok(None);
            }

            let cl = header_content_length(&buf[..pos]).unwrap_or(0);
            if cl > MAX_BODY_BYTES {
                let resp = "HTTP/1.1 413 Payload Too Large\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nPayload Too Large";
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.shutdown().await;
                return Ok(None);
            }

            let body_received = read.saturating_sub(pos + delim_len);
            if body_received >= cl {
                let headers_str = String::from_utf8_lossy(&buf[..pos]);
                let body_str = String::from_utf8_lossy(&buf[pos + delim_len..read]);

                let first = headers_str.lines().next().unwrap_or_default();
                let mut parts = first.split_whitespace();
                let method = parts.next().unwrap_or_default().to_string();
                let path = parts.next().unwrap_or("/").to_string();

                if !path.starts_with("/assets/js/") {
                    info!(
                        "HTTP Request: {} {}\nHeaders:\n{}\nBody:\n{}",
                        method, path, headers_str, body_str
                    );
                }

                return Ok(Some(HttpRequest {
                    method,
                    path,
                    body: body_str.to_string(),
                }));
            }
        } else if read > MAX_HEADER_BYTES {
            let resp = "HTTP/1.1 413 Payload Too Large\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nPayload Too Large";
            let _ = socket.write_all(resp.as_bytes()).await;
            let _ = socket.shutdown().await;
            return Ok(None);
        }

        if read == buf.len() {
            let next_len = (buf.len() * 2).min(MAX_HEADER_BYTES + MAX_BODY_BYTES + 1024);
            if next_len <= buf.len() {
                let resp = "HTTP/1.1 413 Payload Too Large\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nPayload Too Large";
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.shutdown().await;
                return Ok(None);
            }
            buf.resize(next_len, 0);
        }
    }

    Ok(None)
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

#[allow(dead_code)]
pub(crate) fn body_len(bytes: &[u8]) -> usize {
    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|idx| idx + 4)
        .or_else(|| {
            bytes
                .windows(2)
                .position(|window| window == b"\n\n")
                .map(|idx| idx + 2)
        });

    header_end
        .map(|idx| bytes.len().saturating_sub(idx))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::engine::RunnerStatus;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::{mpsc, Mutex};

    #[test]
    fn test_status_reason_phrases() {
        assert_eq!(status_reason_phrase(200), "OK");
        assert_eq!(status_reason_phrase(400), "Bad Request");
        assert_eq!(status_reason_phrase(404), "Not Found");
        assert_eq!(status_reason_phrase(405), "Method Not Allowed");
        assert_eq!(status_reason_phrase(408), "Request Timeout");
        assert_eq!(status_reason_phrase(413), "Payload Too Large");
        assert_eq!(status_reason_phrase(500), "Internal Server Error");
        assert_eq!(status_reason_phrase(999), "Unknown Error");
    }

    #[test]
    fn test_http_parser_line_ending_consistency() {
        let crlf_req = b"POST /test HTTP/1.1\r\nContent-Length: 4\r\n\r\ntest";
        let lf_req = b"POST /test HTTP/1.1\nContent-Length: 4\n\ntest";

        assert_eq!(header_content_length(crlf_req), Some(4));
        assert_eq!(header_content_length(lf_req), Some(4));

        assert_eq!(body_len(crlf_req), 4);
        assert_eq!(body_len(lf_req), 4);
    }

    #[test]
    fn test_non_loopback_binding_rejection() {
        let non_loopback_cfg = RunnerConfig {
            gui_host: "0.0.0.0".to_string(),
            ..Default::default()
        };
        assert!(non_loopback_cfg.validate().is_err());

        let non_loopback_ip = RunnerConfig {
            gui_host: "192.168.1.100".to_string(),
            ..Default::default()
        };
        assert!(non_loopback_ip.validate().is_err());

        let loopback_cfg = RunnerConfig {
            gui_host: "127.0.0.1".to_string(),
            ..Default::default()
        };
        assert!(loopback_cfg.validate().is_ok());
    }

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
            waiting_for_app: std::collections::HashMap::new(),
        }));

        let (exec_tx, _) = mpsc::channel(128);
        let handle = RunnerHandle {
            command_tx: tx,
            exec_tx,
            status,
            runner_config_path: config_path.clone(),
        };

        start_gui_server(handle);

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = reqwest::Client::new();

        let res = client
            .get(format!("http://127.0.0.1:{}/", port))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);

        let res = client
            .get(format!("http://127.0.0.1:{}/status", port))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);

        let res = client
            .get(format!("http://127.0.0.1:{}/run-all", port))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 405);

        let res = client
            .post(format!("http://127.0.0.1:{}/create", port))
            .header("Transfer-Encoding", "chunked")
            .body("0\r\n\r\n")
            .send()
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 400);
    }
}
