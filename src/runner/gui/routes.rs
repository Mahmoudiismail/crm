use anyhow::Result;
use tokio::io::AsyncReadExt;

use crate::runner::config::RunnerConfig;
use crate::runner::engine::RunnerHandle;
use crate::runner::gui::api::*;
use crate::runner::gui::handlers::*;
use crate::runner::gui::helpers::{escape_html, parse_query_string, split_path_and_query};
use crate::runner::gui::response::render_error_page;
use crate::runner::gui::templates::{render_apps_page, render_dashboard, render_task_form};

pub(crate) struct HttpRequest {
    pub method: String,
    pub path: String,
    pub body: String,
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

    Ok(Some(HttpRequest {
        method,
        path,
        body: body.to_string(),
    }))
}

fn header_content_length(bytes: &[u8]) -> Option<usize> {
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

fn body_len(bytes: &[u8]) -> usize {
    bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|idx| bytes.len().saturating_sub(idx + 4))
        .unwrap_or(0)
}

pub(crate) async fn route_request(
    request: &HttpRequest,
    handle: &RunnerHandle,
) -> Result<(u16, &'static str, String)> {
    let (route_path, query_string) = split_path_and_query(&request.path);
    let query = parse_query_string(query_string);

    if request.method == "GET" && route_path == "/" {
        let cfg = RunnerConfig::load(&handle.runner_config_path)?;
        let status = handle.status.lock().await.clone();
        return Ok((
            200,
            "text/html; charset=utf-8",
            render_dashboard(&cfg, &status, query.get("toast").map(String::as_str)),
        ));
    }

    if request.method == "GET" && route_path == "/status" {
        return handle_api_status(handle).await;
    }

    if request.method == "GET" && route_path == "/tasks" {
        return handle_api_tasks(handle).await;
    }

    if request.method == "GET" && route_path == "/new-task" {
        let html = render_task_form("Create Task", "/create", "Create", None, None);
        return Ok((200, "text/html; charset=utf-8", html));
    }

    if request.method == "GET" && route_path.starts_with("/edit/") {
        let task_id = route_path.trim_start_matches("/edit/");
        let cfg = RunnerConfig::load(&handle.runner_config_path)?;
        if let Some(task) = cfg.tasks.iter().find(|t| t.id == task_id) {
            let action = format!("/update/{}", escape_html(task_id));
            let html = render_task_form("Edit Task", &action, "Update", Some(task), None);
            return Ok((200, "text/html; charset=utf-8", html));
        }
        return Ok((
            404,
            "text/html; charset=utf-8",
            render_error_page("Task not found", task_id),
        ));
    }

    if request.method == "POST" && route_path == "/create" {
        let values = parse_query_string(&request.body);
        return handle_create_task(handle, &values).await;
    }

    if request.method == "POST" && route_path.starts_with("/update/") {
        let task_id = route_path.trim_start_matches("/update/");
        let values = parse_query_string(&request.body);
        return handle_update_task(handle, task_id, &values).await;
    }

    if request.method == "GET" && route_path == "/create" {
        return handle_create_task(handle, &query).await;
    }

    if request.method == "GET" && route_path.starts_with("/update/") {
        let task_id = route_path.trim_start_matches("/update/");
        return handle_update_task(handle, task_id, &query).await;
    }

    if request.method == "GET" && route_path.starts_with("/delete/") {
        let task_id = route_path.trim_start_matches("/delete/");
        return handle_delete_task(handle, task_id).await;
    }

    if request.method == "GET" && route_path == "/run-all" {
        return handle_run_all(handle).await;
    }

    if request.method == "GET" && route_path.starts_with("/run/") {
        let task_id = route_path.trim_start_matches("/run/");
        return handle_run_task(handle, task_id).await;
    }

    if request.method == "GET" && route_path.starts_with("/enable/") {
        let task_id = route_path.trim_start_matches("/enable/");
        return handle_enable_task(handle, task_id).await;
    }

    if request.method == "GET" && route_path.starts_with("/disable/") {
        let task_id = route_path.trim_start_matches("/disable/");
        return handle_disable_task(handle, task_id).await;
    }

    if request.method == "GET" && route_path == "/apps" {
        let cfg = RunnerConfig::load(&handle.runner_config_path)?;
        let html = render_apps_page(&cfg.registered_apps);
        return Ok((200, "text/html; charset=utf-8", html));
    }

    if request.method == "POST" && route_path == "/apps/create" {
        let values = parse_query_string(&request.body);
        return handle_create_app(handle, &values).await;
    }

    if request.method == "GET" && route_path.starts_with("/apps/edit/") {
        let app_id = route_path.trim_start_matches("/apps/edit/");
        return handle_edit_app(handle, app_id).await;
    }

    if request.method == "POST" && route_path.starts_with("/apps/update/") {
        let app_id = route_path.trim_start_matches("/apps/update/");
        let values = parse_query_string(&request.body);
        return handle_update_app(handle, app_id, &values).await;
    }

    if request.method == "GET" && route_path.starts_with("/apps/delete/") {
        let app_id = route_path.trim_start_matches("/apps/delete/");
        return handle_delete_app(handle, app_id).await;
    }

    if request.method == "GET" && route_path == "/api/apps/list" {
        return handle_api_apps_list(handle).await;
    }

    if request.method == "GET" && route_path == "/api/apps/manifest" {
        return handle_api_apps_manifest(handle, &query).await;
    }

    Ok((
        404,
        "text/html; charset=utf-8",
        render_error_page("Not found", route_path),
    ))
}
