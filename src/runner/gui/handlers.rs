use anyhow::Result;
use std::collections::HashMap;

use crate::runner::config::RunnerConfig;
use crate::runner::engine::{create_task, delete_task, update_task, RunnerCommand, RunnerHandle};
use crate::runner::gui::forms::build_task_from_values;
use crate::runner::gui::response::{render_error_page, render_redirect_to_dashboard};
use crate::runner::gui::templates::render_app_edit_page;

pub(crate) async fn handle_create_task(
    handle: &RunnerHandle,
    values: &HashMap<String, String>,
) -> Result<(u16, &'static str, String)> {
    let task = build_task_from_values(values, None)?;
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
    let task = build_task_from_values(values, Some(task_id.to_string()))?;
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

pub(crate) async fn handle_run_all(handle: &RunnerHandle) -> Result<(u16, &'static str, String)> {
    handle.command_tx.send(RunnerCommand::RunAllNow).await?;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard("Run-all triggered"),
    ))
}

pub(crate) async fn handle_run_task(
    handle: &RunnerHandle,
    task_id: &str,
) -> Result<(u16, &'static str, String)> {
    handle
        .command_tx
        .send(RunnerCommand::RunTaskNow(task_id.to_string()))
        .await?;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard("Task triggered"),
    ))
}

pub(crate) async fn handle_enable_task(
    handle: &RunnerHandle,
    task_id: &str,
) -> Result<(u16, &'static str, String)> {
    handle
        .command_tx
        .send(RunnerCommand::SetTaskEnabled {
            task_id: task_id.to_string(),
            enabled: true,
        })
        .await?;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard("Task enabled"),
    ))
}

pub(crate) async fn handle_disable_task(
    handle: &RunnerHandle,
    task_id: &str,
) -> Result<(u16, &'static str, String)> {
    handle
        .command_tx
        .send(RunnerCommand::SetTaskEnabled {
            task_id: task_id.to_string(),
            enabled: false,
        })
        .await?;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard("Task disabled"),
    ))
}

pub(crate) async fn handle_create_app(
    handle: &RunnerHandle,
    values: &HashMap<String, String>,
) -> Result<(u16, &'static str, String)> {
    let mut cfg = RunnerConfig::load(&handle.runner_config_path)?;
    let app = crate::runner::config::RegisteredApp {
        id: values
            .get("id")
            .unwrap_or(&String::new())
            .trim()
            .to_string(),
        name: values
            .get("name")
            .unwrap_or(&String::new())
            .trim()
            .to_string(),
        executable_path: values
            .get("executable_path")
            .unwrap_or(&String::new())
            .trim()
            .to_string(),
        config_path: values
            .get("config_path")
            .unwrap_or(&String::new())
            .trim()
            .to_string(),
    };

    if app.id.is_empty() || app.name.is_empty() || app.executable_path.is_empty() {
        return Ok((
            400,
            "text/html; charset=utf-8",
            render_error_page(
                "Invalid app data",
                "ID, Name, and Executable Path are required.",
            ),
        ));
    }

    if cfg.registered_apps.iter().any(|a| a.id == app.id) {
        return Ok((
            400,
            "text/html; charset=utf-8",
            render_error_page("Invalid app data", "An app with this ID already exists."),
        ));
    }

    cfg.registered_apps.push(app);
    cfg.save(&handle.runner_config_path)?;
    Ok((
        200,
        "text/html; charset=utf-8",
        render_redirect_to_dashboard("App registered successfully"),
    ))
}

pub(crate) async fn handle_edit_app(
    handle: &RunnerHandle,
    app_id: &str,
) -> Result<(u16, &'static str, String)> {
    let cfg = RunnerConfig::load(&handle.runner_config_path)?;
    let decoded_app_id = urlencoding::decode(app_id)
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| app_id.to_string());
    if let Some(app) = cfg
        .registered_apps
        .iter()
        .find(|a| a.id == app_id || a.id == decoded_app_id)
    {
        let html = render_app_edit_page(app);
        return Ok((200, "text/html; charset=utf-8", html));
    }
    Ok((
        404,
        "text/html; charset=utf-8",
        render_error_page("App not found", app_id),
    ))
}

pub(crate) async fn handle_update_app(
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
        return Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard("App updated"),
        ));
    }
    Ok((
        404,
        "text/html; charset=utf-8",
        render_error_page("App not found", app_id),
    ))
}

pub(crate) async fn handle_delete_app(
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
