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
use tracing::{error, info, warn};

async fn load_runner_config(handle: &RunnerHandle) -> Result<RunnerConfig> {
    crate::runner::engine::dispatcher::helpers::load_config(&handle.runner_config_path).await
}

fn task_config_matches_readback(expected: &RunnerTask, saved: &RunnerTask) -> bool {
    fn comparable_task(task: &RunnerTask) -> Option<serde_json::Value> {
        let mut value = serde_json::to_value(task).ok()?;
        let object = value.as_object_mut()?;
        object.remove("last_run_at");
        object.remove("last_status");
        object.remove("next_run_at");

        if let Some(schedules) = object
            .get_mut("schedules")
            .and_then(serde_json::Value::as_array_mut)
        {
            for schedule in schedules {
                if let Some(schedule) = schedule.as_object_mut() {
                    schedule.remove("next_run_at");
                }
            }
        }

        Some(value)
    }

    match (comparable_task(expected), comparable_task(saved)) {
        (Some(expected), Some(saved)) => expected == saved,
        _ => false,
    }
}

pub(crate) async fn handle_dashboard(
    handle: &RunnerHandle,
    query: &HashMap<String, String>,
) -> Result<(u16, &'static str, String)> {
    let cfg = load_runner_config(handle).await?;
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
    let cfg = load_runner_config(handle).await?;
    let body = serde_json::to_string_pretty(&cfg.tasks)?;
    Ok((200, "application/json", body))
}

pub(crate) async fn handle_new_task_page(
    handle: &RunnerHandle,
) -> Result<(u16, &'static str, String)> {
    let cfg = load_runner_config(handle).await?;
    let html = render_task_form(
        &cfg.working_hours_profiles,
        "Create Task",
        "/create",
        "Create",
        None,
        None,
    );
    Ok((200, "text/html; charset=utf-8", html))
}

pub(crate) async fn handle_edit_task_page(
    handle: &RunnerHandle,
    task_id: &str,
) -> Result<(u16, &'static str, String)> {
    let cfg = load_runner_config(handle).await?;
    if let Some(task) = cfg.tasks.iter().find(|t| t.id == task_id) {
        let action = format!("/update/{}", escape_html(task_id));
        let html = render_task_form(
            &cfg.working_hours_profiles,
            "Edit Task",
            &action,
            "Update",
            Some(task),
            None,
        );
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
    let cfg = load_runner_config(handle).await?;
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
    let cfg = load_runner_config(handle).await?;
    let task = build_task_from_values(
        values,
        Some(task_id.to_string()),
        &cfg.working_hours_profiles,
    )?;
    let mut expected_task = task.clone();
    crate::runner::config::normalize_and_validate_task(&mut expected_task, &cfg)?;

    let mut submitted_fields: Vec<&str> = values.keys().map(String::as_str).collect();
    submitted_fields.sort_unstable();
    info!(
        task_id = %task_id,
        submitted_fields = ?submitted_fields,
        expected_enabled = task.enabled,
        expected_timeout_seconds = task.timeout_seconds,
        expected_schedule_count = task.schedules.len(),
        expected_step_count = task.steps.len(),
        expected_post_run_step_count = task.post_run_steps.len(),
        "Task update validated; saving configuration"
    );

    update_task(&handle.runner_config_path, task_id, task).await?;

    let saved_cfg = match load_runner_config(handle).await {
        Ok(cfg) => cfg,
        Err(error) => {
            error!(
                task_id = %task_id,
                error = %error,
                "Failed to read back task configuration after update"
            );
            return Ok((
                500,
                "text/html; charset=utf-8",
                render_error_page("Save verification failed", "Could not reload the saved task configuration."),
            ));
        }
    };
    let Some(saved_task) = saved_cfg.tasks.iter().find(|saved| saved.id == task_id) else {
        error!(
            task_id = %task_id,
            "Task configuration missing after successful update"
        );
        return Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Save verification failed", "The task was missing after saving."),
        ));
    };

    if !task_config_matches_readback(&expected_task, saved_task) {
        error!(
            task_id = %task_id,
            "Saved task configuration does not match the submitted editable fields"
        );
        return Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page(
                "Save verification failed",
                "The saved task settings did not match the submitted settings. Reload the task and try again.",
            ),
        ));
    }

    info!(
        task_id = %task_id,
        readback_matches_editable_fields = true,
        "Task configuration read-back completed"
    );

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
    let (tx, rx) = tokio::sync::oneshot::channel();
    if handle
        .command_tx
        .send(RunnerCommand::RunTaskNowWithReply {
            task_id: task_id.to_string(),
            is_manual: true,
            reply: tx,
        })
        .await
        .is_err()
    {
        return Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Scheduler channel closed."),
        ));
    }

    match rx.await {
        Ok(Ok(())) => Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard(&format!("Triggered task {}", escape_html(task_id))),
        )),
        Ok(Err(error)) => Ok((
            409,
            "text/html; charset=utf-8",
            render_error_page("Task not triggered", &error),
        )),
        Err(_) => Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Scheduler did not confirm the task request."),
        )),
    }
}

pub(crate) async fn handle_enable_task(
    handle: &RunnerHandle,
    task_id: &str,
    enabled: bool,
) -> Result<(u16, &'static str, String)> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    if handle
        .command_tx
        .send(RunnerCommand::SetTaskEnabled {
            task_id: task_id.to_string(),
            enabled,
            reply: tx,
        })
        .await
        .is_err()
    {
        return Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Scheduler channel closed."),
        ));
    }
    match rx.await {
        Ok(Ok(_)) => Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard(if enabled {
                "Task enabled"
            } else {
                "Task disabled"
            }),
        )),
        _ => Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Failed to toggle task."),
        )),
    }
}

pub(crate) async fn handle_wh_page(handle: &RunnerHandle) -> Result<(u16, &'static str, String)> {
    let cfg = load_runner_config(handle).await?;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_wh_page(&cfg.working_hours_profiles),
    ))
}

pub(crate) async fn handle_wh_new_page(
    _handle: &RunnerHandle,
) -> Result<(u16, &'static str, String)> {
    Ok((200, "text/html; charset=utf-8", render_wh_edit_page(None)))
}

pub(crate) async fn handle_wh_edit_page(
    handle: &RunnerHandle,
    id: &str,
) -> Result<(u16, &'static str, String)> {
    let cfg = load_runner_config(handle).await?;
    if let Some(profile) = cfg.working_hours_profiles.iter().find(|p| p.id == id) {
        Ok((
            200,
            "text/html; charset=utf-8",
            render_wh_edit_page(Some(profile)),
        ))
    } else {
        Ok((
            404,
            "text/html; charset=utf-8",
            render_error_page("Not Found", "Profile not found"),
        ))
    }
}

pub(crate) async fn handle_wh_create(
    handle: &RunnerHandle,
    values: &HashMap<String, String>,
) -> Result<(u16, &'static str, String)> {
    let mut id = values.get("id").unwrap_or(&"".to_string()).clone();
    if id.is_empty() {
        id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();
    }
    let name = values.get("name").unwrap_or(&"".to_string()).clone();
    let mut days = HashMap::new();
    for day in &[
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ] {
        let start = values
            .get(&format!("{}_start", day))
            .unwrap_or(&"".to_string())
            .clone();
        let end = values
            .get(&format!("{}_end", day))
            .unwrap_or(&"".to_string())
            .clone();
        if !start.is_empty() && !end.is_empty() {
            days.insert(day.to_string(), WorkingHours { start, end });
        }
    }
    let profile = WorkingHoursProfile { id, name, days };
    let (tx, rx) = tokio::sync::oneshot::channel();
    if handle
        .command_tx
        .send(RunnerCommand::CreateWorkingHoursProfile { profile, reply: tx })
        .await
        .is_err()
    {
        return Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Scheduler channel closed."),
        ));
    }
    match rx.await {
        Ok(Ok(_)) => Ok((
            303,
            "text/html; charset=utf-8",
            "<meta http-equiv=\"refresh\" content=\"0; url=/working-hours\">".to_string(),
        )),
        _ => Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Failed to create profile."),
        )),
    }
}

pub(crate) async fn handle_wh_update(
    handle: &RunnerHandle,
    _id: &str,
    values: &HashMap<String, String>,
) -> Result<(u16, &'static str, String)> {
    let mut id = values.get("id").unwrap_or(&"".to_string()).clone();
    if id.is_empty() {
        id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();
    }
    let name = values.get("name").unwrap_or(&"".to_string()).clone();
    let mut days = HashMap::new();
    for day in &[
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ] {
        let start = values
            .get(&format!("{}_start", day))
            .unwrap_or(&"".to_string())
            .clone();
        let end = values
            .get(&format!("{}_end", day))
            .unwrap_or(&"".to_string())
            .clone();
        if !start.is_empty() && !end.is_empty() {
            days.insert(day.to_string(), WorkingHours { start, end });
        }
    }
    let profile = WorkingHoursProfile { id, name, days };
    let (tx, rx) = tokio::sync::oneshot::channel();
    if handle
        .command_tx
        .send(RunnerCommand::UpdateWorkingHoursProfile { profile, reply: tx })
        .await
        .is_err()
    {
        return Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Scheduler channel closed."),
        ));
    }
    match rx.await {
        Ok(Ok(_)) => Ok((
            303,
            "text/html; charset=utf-8",
            "<meta http-equiv=\"refresh\" content=\"0; url=/working-hours\">".to_string(),
        )),
        _ => Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Failed to update profile."),
        )),
    }
}

pub(crate) async fn handle_wh_delete(
    handle: &RunnerHandle,
    id: &str,
) -> Result<(u16, &'static str, String)> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    if handle
        .command_tx
        .send(RunnerCommand::DeleteWorkingHoursProfile {
            profile_id: id.to_string(),
            reply: tx,
        })
        .await
        .is_err()
    {
        return Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Scheduler channel closed."),
        ));
    }
    match rx.await {
        Ok(Ok(_)) => Ok((
            303,
            "text/html; charset=utf-8",
            "<meta http-equiv=\"refresh\" content=\"0; url=/working-hours\">".to_string(),
        )),
        _ => Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Failed to delete profile."),
        )),
    }
}

pub(crate) async fn handle_apps_page(handle: &RunnerHandle) -> Result<(u16, &'static str, String)> {
    let cfg = load_runner_config(handle).await?;
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
    let cfg = load_runner_config(handle).await?;
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

pub(crate) async fn handle_apps_create(
    handle: &RunnerHandle,
    values: &HashMap<String, String>,
) -> Result<(u16, &'static str, String)> {
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
        allow_concurrent_tasks: crate::runner::gui::forms::parse_checkbox(
            values,
            "allow_concurrent_tasks",
        ),
    };

    let (tx, rx) = tokio::sync::oneshot::channel();
    if handle
        .command_tx
        .send(RunnerCommand::CreateRegisteredApp { app, reply: tx })
        .await
        .is_err()
    {
        return Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Scheduler channel closed."),
        ));
    }
    match rx.await {
        Ok(Ok(_)) => Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard("App registered"),
        )),
        _ => Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Failed to register app."),
        )),
    }
}

pub(crate) async fn handle_apps_update(
    handle: &RunnerHandle,
    app_id: &str,
    values: &HashMap<String, String>,
) -> Result<(u16, &'static str, String)> {
    let decoded_app_id = urlencoding::decode(app_id)
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| app_id.to_string());

    let app = crate::runner::config::RegisteredApp {
        id: decoded_app_id.clone(),
        name: values
            .get("name")
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
        executable_path: values
            .get("executable_path")
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
        config_path: values
            .get("config_path")
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
        allow_concurrent_tasks: crate::runner::gui::forms::parse_checkbox(
            values,
            "allow_concurrent_tasks",
        ),
    };

    let (tx, rx) = tokio::sync::oneshot::channel();
    if handle
        .command_tx
        .send(RunnerCommand::UpdateRegisteredApp { app, reply: tx })
        .await
        .is_err()
    {
        return Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Scheduler channel closed."),
        ));
    }
    match rx.await {
        Ok(Ok(_)) => Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard("App updated"),
        )),
        _ => Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Failed to update app."),
        )),
    }
}

pub(crate) async fn handle_apps_delete(
    handle: &RunnerHandle,
    app_id: &str,
) -> Result<(u16, &'static str, String)> {
    let decoded_app_id = urlencoding::decode(app_id)
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| app_id.to_string());

    let (tx, rx) = tokio::sync::oneshot::channel();
    if handle
        .command_tx
        .send(RunnerCommand::DeleteRegisteredApp {
            app_id: decoded_app_id,
            reply: tx,
        })
        .await
        .is_err()
    {
        return Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Scheduler channel closed."),
        ));
    }
    match rx.await {
        Ok(Ok(_)) => Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard("App deleted"),
        )),
        _ => Ok((
            500,
            "text/html; charset=utf-8",
            render_error_page("Error", "Failed to delete app."),
        )),
    }
}

pub(crate) async fn handle_api_apps_list(
    handle: &RunnerHandle,
) -> Result<(u16, &'static str, String)> {
    let cfg = load_runner_config(handle).await?;
    let body = serde_json::to_string(&cfg.registered_apps)?;
    Ok((200, "application/json", body))
}

pub(crate) async fn handle_api_task_preview(
    handle: &RunnerHandle,
    values: &HashMap<String, String>,
) -> Result<(u16, &'static str, String)> {
    let cfg = match load_runner_config(handle).await {
        Ok(cfg) => cfg,
        Err(error) => {
            let body = serde_json::json!({
                "success": false,
                "error": format!("Failed to load runner configuration: {}", error)
            });
            return Ok((500, "application/json", body.to_string()));
        }
    };
    let task = match build_task_from_values(values, None, &cfg.working_hours_profiles) {
        Ok(t) => t,
        Err(e) => {
            let json_err = serde_json::json!({
                "success": false,
                "error": format!("Task validation error: {}", e)
            });
            return Ok((200, "application/json", json_err.to_string()));
        }
    };

    let period_mode_str = values
        .get("period_mode")
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default();
    let period_mode = match period_mode_str.as_str() {
        "monthly" => PeriodMode::Monthly,
        "quarterly" => PeriodMode::Quarterly,
        "a_month" => PeriodMode::AMonth,
        "a_quarter" => PeriodMode::AQuarter,
        _ => PeriodMode::Custom,
    };

    let start_date = values
        .get("start_date")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let end_date = values
        .get("end_date")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let app_id = values.get("app_id").cloned().unwrap_or_default();

    let mut app_spec = ExternalAppSpec {
        app_id,
        args: HashMap::new(),
        period_mode,
        start_date,
        end_date,
        start_date_arg: values
            .get("start_date_arg")
            .cloned()
            .filter(|s| !s.is_empty()),
        end_date_arg: values
            .get("end_date_arg")
            .cloned()
            .filter(|s| !s.is_empty()),
    };
    if let Some(args_str) = values.get("args") {
        if let Ok(args_map) = serde_json::from_str::<HashMap<String, String>>(args_str) {
            app_spec.args = args_map;
        }
    }

    let now = Utc::now();
    match generate_upcoming_executions_for_app(&task, &app_spec, now, 10) {
        Ok(occurrences) => {
            let items: Vec<_> = occurrences
                .into_iter()
                .map(|occ| {
                    let local_dt = occ.scheduled_at.with_timezone(&Local);
                    serde_json::json!({
                        "period_start": occ.period.start_date.format("%Y-%m-%d").to_string(),
                        "period_end": occ.period.end_date.format("%Y-%m-%d").to_string(),
                        "scheduled_at_iso": occ.scheduled_at.to_rfc3339(),
                        "scheduled_at_formatted": local_dt.format("%b %d, %Y %I:%M %p local").to_string(),
                    })
                })
                .collect();

            let res = serde_json::json!({
                "success": true,
                "executions": items
            });
            Ok((200, "application/json", res.to_string()))
        }
        Err(e) => {
            let json_err = serde_json::json!({
                "success": false,
                "error": format!("Preview calculation error: {}", e)
            });
            Ok((200, "application/json", json_err.to_string()))
        }
    }
}

pub(crate) async fn handle_api_apps_manifest(
    handle: &RunnerHandle,
    app_id: &str,
) -> Result<(u16, &'static str, String)> {
    let cfg = load_runner_config(handle).await?;
    if let Some(app) = cfg.registered_apps.iter().find(|a| a.id == app_id) {
        if app.executable_path.is_empty() {
            return Ok((
                400,
                "application/json",
                "{\"error\": \"App executable path is missing\"}".to_string(),
            ));
        }

        let mut child_cmd = tokio::process::Command::new(&app.executable_path);
        child_cmd
            .arg("--manifest")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        let spawn_res = child_cmd.spawn();
        let output_res = match spawn_res {
            Ok(child) => {
                let child_pid = child.id();
                match tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    child.wait_with_output(),
                )
                .await
                {
                    Ok(res) => res,
                    Err(_) => {
                        if let Some(pid) = child_pid {
                            #[cfg(target_os = "windows")]
                            {
                                let _ = tokio::process::Command::new("taskkill")
                                    .args(["/F", "/T", "/PID", &pid.to_string()])
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .status()
                                    .await;
                            }
                            #[cfg(not(target_os = "windows"))]
                            {
                                let _ = tokio::process::Command::new("pkill")
                                    .args(["-P", &pid.to_string()])
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .status()
                                    .await;
                            }
                        }
                        return Ok((
                            500,
                            "application/json",
                            "{\"error\": \"App manifest execution timed out\"}".to_string(),
                        ));
                    }
                }
            }
            Err(e) => Err(e),
        };

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
