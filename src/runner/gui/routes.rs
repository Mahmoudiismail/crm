#![allow(unused_imports)]
use super::handlers::*;
use super::helpers::*;
use super::templates::*;
use super::HttpRequest;
use crate::runner::config::*;
use crate::runner::engine::*;
use anyhow::{Context, Result};
use chrono::{Local, Utc};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info};

pub(crate) async fn route_request(
    request: &HttpRequest,
    handle: &RunnerHandle,
) -> Result<(u16, &'static str, String)> {
    let (route_path, query_string) = split_path_and_query(&request.path);
    let query = parse_query_string(query_string);

    if request.method == "GET" && route_path == "/" {
        return handle_dashboard(handle, &query).await;
    }
    if request.method == "GET" && route_path == "/status" {
        return handle_status_api(handle).await;
    }
    if request.method == "GET" && route_path == "/tasks" {
        return handle_tasks_api(handle).await;
    }
    if request.method == "GET" && route_path == "/new-task" {
        return handle_new_task_page().await;
    }
    if request.method == "GET" && route_path.starts_with("/edit/") {
        let task_id = route_path.trim_start_matches("/edit/");
        return handle_edit_task_page(handle, task_id).await;
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
        return handle_enable_task(handle, task_id, true).await;
    }
    if request.method == "GET" && route_path.starts_with("/disable/") {
        let task_id = route_path.trim_start_matches("/disable/");
        return handle_enable_task(handle, task_id, false).await;
    }
    if request.method == "GET" && route_path == "/reload" {
        return handle_reload(handle).await;
    }
    if request.method == "GET" && route_path == "/apps" {
        return handle_apps_page(handle).await;
    }
    if request.method == "GET" && route_path.starts_with("/apps/edit/") {
        let app_id = route_path.trim_start_matches("/apps/edit/");
        return handle_app_edit_page(handle, app_id).await;
    }
    if request.method == "POST" && route_path == "/apps/create" {
        let values = parse_query_string(&request.body);
        return handle_apps_create(handle, &values).await;
    }
    if request.method == "POST" && route_path.starts_with("/apps/update/") {
        let app_id = route_path.trim_start_matches("/apps/update/");
        let values = parse_query_string(&request.body);
        return handle_apps_update(handle, app_id, &values).await;
    }
    if request.method == "GET" && route_path.starts_with("/apps/delete/") {
        let app_id = route_path.trim_start_matches("/apps/delete/");
        return handle_apps_delete(handle, app_id).await;
    }
    if request.method == "GET" && route_path == "/api/apps/list" {
        return handle_api_apps_list(handle).await;
    }
    if request.method == "GET" && route_path == "/api/apps/manifest" {
        let app_id = query.get("app_id").map(|s| s.as_str()).unwrap_or("");
        return handle_api_apps_manifest(handle, app_id).await;
    }
    if request.method == "GET" && route_path == "/assets/js/common.js" {
        return Ok((
            200,
            "application/javascript",
            include_str!("../assets/js/common.js").to_string(),
        ));
    }
    if request.method == "GET" && route_path == "/assets/js/api.js" {
        return Ok((
            200,
            "application/javascript",
            include_str!("../assets/js/api.js").to_string(),
        ));
    }
    if request.method == "GET" && route_path == "/assets/js/validation.js" {
        return Ok((
            200,
            "application/javascript",
            include_str!("../assets/js/validation.js").to_string(),
        ));
    }
    if request.method == "GET" && route_path == "/assets/js/notifications.js" {
        return Ok((
            200,
            "application/javascript",
            include_str!("../assets/js/notifications.js").to_string(),
        ));
    }
    if request.method == "GET" && route_path == "/assets/js/forms.js" {
        return Ok((
            200,
            "application/javascript",
            include_str!("../assets/js/forms.js").to_string(),
        ));
    }

    Ok((
        404,
        "text/html; charset=utf-8",
        render_error_page("Not found", route_path),
    ))
}
