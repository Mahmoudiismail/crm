use anyhow::Result;
use std::collections::HashMap;
use tracing::error;

use crate::runner::config::RunnerConfig;
use crate::runner::engine::RunnerHandle;

pub(crate) async fn handle_api_status(
    handle: &RunnerHandle,
) -> Result<(u16, &'static str, String)> {
    let status = handle.status.lock().await.clone();
    let body = serde_json::to_string_pretty(&status)?;
    Ok((200, "application/json", body))
}

pub(crate) async fn handle_api_tasks(handle: &RunnerHandle) -> Result<(u16, &'static str, String)> {
    let cfg = RunnerConfig::load(&handle.runner_config_path)?;
    let body = serde_json::to_string_pretty(&cfg.tasks)?;
    Ok((200, "application/json", body))
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
    query: &HashMap<String, String>,
) -> Result<(u16, &'static str, String)> {
    let app_id = query.get("app_id").unwrap_or(&String::new()).to_string();
    let cfg = RunnerConfig::load(&handle.runner_config_path)?;

    if let Some(app) = cfg.registered_apps.iter().find(|a| a.id == app_id) {
        let executable = crate::runner::engine::resolve_executable(&app.executable_path);
        let mut cmd = tokio::process::Command::new(executable);
        cmd.arg("--manifest");

        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(0x08000000);
        }

        let output_result = cmd.output().await;

        match output_result {
            Ok(output) if output.status.success() => {
                let json = String::from_utf8_lossy(&output.stdout).to_string();
                Ok((200, "application/json", json))
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                error!("App {} returned error for --manifest: {}", app.id, err);
                Ok((
                    500,
                    "application/json",
                    r#"{"error":"App returned non-zero status"}"#.to_string(),
                ))
            }
            Err(e) => {
                error!("Failed to execute app {} for --manifest: {}", app.id, e);
                Ok((
                    500,
                    "application/json",
                    r#"{"error":"Failed to execute app"}"#.to_string(),
                ))
            }
        }
    } else {
        Ok((
            404,
            "application/json",
            r#"{"error":"App not found"}"#.to_string(),
        ))
    }
}
