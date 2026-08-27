#![allow(unused_imports)]
use super::forms::*;
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

pub(crate) async fn handle_dashboard(
    handle: &RunnerHandle,
    query: &HashMap<String, String>,
) -> Result<(u16, &'static str, String)> {
    let cfg = RunnerConfig::load(&handle.runner_config_path)?;
    let status = handle.status.lock().await.clone();
    Ok((
        200,
        "text/html; charset=utf-8",
        render_dashboard(&cfg, &status, query.get("toast").map(String::as_str)),
    ))
}

pub(crate) async fn handle_status_api(
    handle: &RunnerHandle,
) -> Result<(u16, &'static str, String)> {
    let status = handle.status.lock().await.clone();
    let body = serde_json::to_string_pretty(&status)?;
    Ok((200, "application/json", body))
}

pub(crate) async fn handle_tasks_api(handle: &RunnerHandle) -> Result<(u16, &'static str, String)> {
    let cfg = RunnerConfig::load(&handle.runner_config_path)?;
    let body = serde_json::to_string_pretty(&cfg.tasks)?;
    Ok((200, "application/json", body))
}

pub(crate) async fn handle_new_task_page(handle: &RunnerHandle) -> Result<(u16, &'static str, String)> {
    let cfg = RunnerConfig::load(&handle.runner_config_path).unwrap_or_default();
    let html = render_task_form(&cfg.working_hours_profiles, "Create Task", "/create", "Create", None, None);
    Ok((200, "text/html; charset=utf-8", html))
}

pub(crate) async fn handle_edit_task_page(
    handle: &RunnerHandle,
    task_id: &str,
) -> Result<(u16, &'static str, String)> {
    let cfg = RunnerConfig::load(&handle.runner_config_path)?;
    if let Some(task) = cfg.tasks.iter().find(|t| t.id == task_id) {
        let action = format!("/update/{}", escape_html(task_id));
        let cfg = RunnerConfig::load(&handle.runner_config_path).unwrap_or_default();
    let html = render_task_form(&cfg.working_hours_profiles, "Edit Task", &action, "Update", Some(task), None);
        return Ok((200, "text/html; charset=utf-8", html));
    }
    Ok((
        404,
        "text/html; charset=utf-8",
        render_error_page("Task not found", task_id),
    ))
}

pub(crate) async fn handle_create_task(
    handle: &RunnerHandle,
    values: &HashMap<String, String>,
) -> Result<(u16, &'static str, String)> {

    let cfg = RunnerConfig::load(&handle.runner_config_path).unwrap_or_default();
    let task = build_task_from_values(values, None, &cfg.working_hours_profiles)?;

    create_task(&handle.runner_config_path, task).await?;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard("Task created"),
    ))
}

pub(crate) async fn handle_update_task(
    handle: &RunnerHandle,
    task_id: &str,
    values: &HashMap<String, String>,
) -> Result<(u16, &'static str, String)> {

    let cfg = RunnerConfig::load(&handle.runner_config_path).unwrap_or_default();
    let task = build_task_from_values(values, Some(task_id.to_string()), &cfg.working_hours_profiles)?;

    update_task(&handle.runner_config_path, task_id, task).await?;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard("Task updated"),
    ))
}

pub(crate) async fn handle_delete_task(
    handle: &RunnerHandle,
    task_id: &str,
) -> Result<(u16, &'static str, String)> {
    delete_task(&handle.runner_config_path, task_id).await?;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard("Task deleted"),
    ))
}

pub(crate) async fn handle_run_task(
    handle: &RunnerHandle,
    task_id: &str,
) -> Result<(u16, &'static str, String)> {
    let _ = handle
        .command_tx
        .send(RunnerCommand::RunTaskNow {
            task_id: task_id.to_string(),
            is_manual: true,
        })
        .await;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard(&format!("Triggered task {}", escape_html(task_id))),
    ))
}

pub(crate) async fn handle_enable_task(
    handle: &RunnerHandle,
    task_id: &str,
    enabled: bool,
) -> Result<(u16, &'static str, String)> {
    let _ = handle
        .command_tx
        .send(RunnerCommand::SetTaskEnabled {
            task_id: task_id.to_string(),
            enabled,
        })
        .await;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard(if enabled {
            "Task enabled"
        } else {
            "Task disabled"
        }),
    ))
}


pub(crate) async fn handle_wh_page(handle: &RunnerHandle) -> Result<(u16, &'static str, String)> {
    let cfg = RunnerConfig::load(&handle.runner_config_path)?;
    Ok((200, "text/html; charset=utf-8", render_wh_page(&cfg.working_hours_profiles)))
}

pub(crate) async fn handle_wh_new_page(_handle: &RunnerHandle) -> Result<(u16, &'static str, String)> {
    Ok((200, "text/html; charset=utf-8", render_wh_edit_page(None)))
}

pub(crate) async fn handle_wh_edit_page(handle: &RunnerHandle, id: &str) -> Result<(u16, &'static str, String)> {
    let cfg = RunnerConfig::load(&handle.runner_config_path)?;
    if let Some(profile) = cfg.working_hours_profiles.iter().find(|p| p.id == id) {
        Ok((200, "text/html; charset=utf-8", render_wh_edit_page(Some(profile))))
    } else {
        Ok((404, "text/html; charset=utf-8", render_error_page("Not Found", "Profile not found")))
    }
}

pub(crate) async fn handle_wh_create(handle: &RunnerHandle, values: &HashMap<String, String>) -> Result<(u16, &'static str, String)> {
    let id = values.get("id").unwrap_or(&"".to_string()).clone();
    let name = values.get("name").unwrap_or(&"".to_string()).clone();
    let mut days = HashMap::new();
    for day in &["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"] {
        let start = values.get(&format!("{}_start", day)).unwrap_or(&"".to_string()).clone();
        let end = values.get(&format!("{}_end", day)).unwrap_or(&"".to_string()).clone();
        if !start.is_empty() && !end.is_empty() {
            days.insert(day.to_string(), WorkingHours { start, end });
        }
    }
    let profile = WorkingHoursProfile { id, name, days };
    let _ = handle.command_tx.send(RunnerCommand::CreateWorkingHoursProfile { profile }).await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    Ok((303, "text/html; charset=utf-8", format!("<meta http-equiv=\"refresh\" content=\"0; url=/working-hours\">")))
}

pub(crate) async fn handle_wh_update(handle: &RunnerHandle, _id: &str, values: &HashMap<String, String>) -> Result<(u16, &'static str, String)> {
    let id = values.get("id").unwrap_or(&"".to_string()).clone();
    let name = values.get("name").unwrap_or(&"".to_string()).clone();
    let mut days = HashMap::new();
    for day in &["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"] {
        let start = values.get(&format!("{}_start", day)).unwrap_or(&"".to_string()).clone();
        let end = values.get(&format!("{}_end", day)).unwrap_or(&"".to_string()).clone();
        if !start.is_empty() && !end.is_empty() {
            days.insert(day.to_string(), WorkingHours { start, end });
        }
    }
    let profile = WorkingHoursProfile { id, name, days };
    let _ = handle.command_tx.send(RunnerCommand::UpdateWorkingHoursProfile { profile }).await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    Ok((303, "text/html; charset=utf-8", format!("<meta http-equiv=\"refresh\" content=\"0; url=/working-hours\">")))
}

pub(crate) async fn handle_wh_delete(handle: &RunnerHandle, id: &str) -> Result<(u16, &'static str, String)> {
    let _ = handle.command_tx.send(RunnerCommand::DeleteWorkingHoursProfile { profile_id: id.to_string() }).await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    Ok((303, "text/html; charset=utf-8", format!("<meta http-equiv=\"refresh\" content=\"0; url=/working-hours\">")))
}

pub(crate) async fn handle_apps_page(handle: &RunnerHandle) -> Result<(u16, &'static str, String)> {
    let cfg = RunnerConfig::load(&handle.runner_config_path)?;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_apps_page(&cfg.registered_apps),
    ))
}

pub(crate) async fn handle_app_edit_page(
    handle: &RunnerHandle,
    app_id: &str,
) -> Result<(u16, &'static str, String)> {
    let cfg = RunnerConfig::load(&handle.runner_config_path)?;
    if let Some(app) = cfg.registered_apps.iter().find(|a| a.id == app_id) {
        return Ok((200, "text/html; charset=utf-8", render_app_edit_page(app)));
    }
    Ok((
        404,
        "text/html; charset=utf-8",
        render_error_page("App not found", app_id),
    ))
}

pub(crate) async fn handle_run_all(handle: &RunnerHandle) -> Result<(u16, &'static str, String)> {
    handle
        .command_tx
        .send(RunnerCommand::RunAllNow { is_manual: true })
        .await?;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard("Run-all triggered"),
    ))
}

pub(crate) async fn handle_reload(_handle: &RunnerHandle) -> Result<(u16, &'static str, String)> {
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard("Configuration reloaded"),
    ))
}

pub(crate) async fn handle_apps_create(
    handle: &RunnerHandle,
    values: &HashMap<String, String>,
) -> Result<(u16, &'static str, String)> {
    let mut cfg = RunnerConfig::load(&handle.runner_config_path)?;
    let app = crate::runner::config::RegisteredApp {
        id: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string(),
        name: values
            .get("name")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "New App".to_string()),
        executable_path: values
            .get("executable_path")
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
        config_path: values
            .get("config_path")
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
    };
    cfg.registered_apps.push(app);
    cfg.save(&handle.runner_config_path)?;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard("App registered"),
    ))
}

pub(crate) async fn handle_apps_update(
    handle: &RunnerHandle,
    app_id: &str,
    values: &HashMap<String, String>,
) -> Result<(u16, &'static str, String)> {
    let mut cfg = RunnerConfig::load(&handle.runner_config_path)?;
    let decoded_app_id = urlencoding::decode(app_id)
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| app_id.to_string());
    if let Some(app) = cfg
        .registered_apps
        .iter_mut()
        .find(|a| a.id == app_id || a.id == decoded_app_id)
    {
        app.name = values
            .get("name")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| app.name.clone());
        app.executable_path = values
            .get("executable_path")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| app.executable_path.clone());
        app.config_path = values
            .get("config_path")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| app.config_path.clone());
        cfg.save(&handle.runner_config_path)?;
    }
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard("App updated"),
    ))
}

pub(crate) async fn handle_apps_delete(
    handle: &RunnerHandle,
    app_id: &str,
) -> Result<(u16, &'static str, String)> {
    let mut cfg = RunnerConfig::load(&handle.runner_config_path)?;
    cfg.registered_apps.retain(|a| a.id != app_id);
    cfg.save(&handle.runner_config_path)?;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard("App deleted"),
    ))
}

pub(crate) async fn handle_api_apps_list(
    handle: &RunnerHandle,
) -> Result<(u16, &'static str, String)> {
    let cfg = RunnerConfig::load(&handle.runner_config_path)?;
    let body = serde_json::to_string(&cfg.registered_apps)?;
    Ok((200, "application/json", body))
}

pub(crate) async fn handle_api_apps_manifest(
    handle: &RunnerHandle,
    app_id: &str,
) -> Result<(u16, &'static str, String)> {
    let cfg = RunnerConfig::load(&handle.runner_config_path)?;
    if let Some(app) = cfg.registered_apps.iter().find(|a| a.id == app_id) {
        if app.executable_path.is_empty() {
            return Ok((
                400,
                "application/json",
                "{\"error\": \"App executable path is missing\"}".to_string(),
            ));
        }

        let output_res = std::process::Command::new(&app.executable_path)
            .arg("--manifest")
            .output();

        match output_res {
            Ok(output) => {
                if output.status.success() {
                    let mut body = String::from_utf8_lossy(&output.stdout).to_string();
                    if body.trim().is_empty() {
                        body = "{}".to_string();
                    }
                    Ok((200, "application/json", body))
                } else {
                    let err_msg = String::from_utf8_lossy(&output.stderr);
                    Ok((
                        500,
                        "application/json",
                        format!(
                            "{{\"error\": \"App returned error: {}\"}}",
                            js_escape(&err_msg)
                        ),
                    ))
                }
            }
            Err(e) => Ok((
                500,
                "application/json",
                format!(
                    "{{\"error\": \"Failed to execute app: {}\"}}",
                    js_escape(&e.to_string())
                ),
            )),
        }
    } else {
        Ok((
            404,
            "application/json",
            "{\"error\": \"App not found\"}".to_string(),
        ))
    }
}
