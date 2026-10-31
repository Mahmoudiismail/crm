#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use crm_tool::manifest::{AppArg, AppManifest};
#[cfg(target_os = "windows")]
use crm_tool::runner::engine::RunnerHandle;
use crm_tool::runner::engine::{start_scheduler, RunnerCommand};
use crm_tool::runner::gui::start_gui_server;
use crm_tool::utils::{
    executable_dir, intercept_manifest, parse_log_level, setup_logging_with_levels_and_header,
    InterceptResult,
};
#[cfg(target_os = "windows")]
use muda::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::time::Duration;
#[cfg(target_os = "windows")]
use tracing::error;
use tracing::info;
#[cfg(target_os = "windows")]
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
#[cfg(target_os = "windows")]
use winit::application::ApplicationHandler;
#[cfg(target_os = "windows")]
use winit::event::WindowEvent;
#[cfg(target_os = "windows")]
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
#[cfg(target_os = "windows")]
use winit::window::WindowId;

#[derive(Parser)]
#[command(name = "runner", about = "Runner daemon")]
struct RunnerCliOptions {
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
    #[arg(long, hide = true)]
    manifest: bool,
}

fn get_manifest() -> AppManifest {
    AppManifest {
        name: "runner".to_string(),
        description: "Runner daemon".to_string(),
        arguments: vec![AppArg::new("--config", crm_tool::manifest::ArgType::String)],
    }
}

fn resolve_config_path(config_arg: Option<&Path>, exe_dir: &Path) -> PathBuf {
    match config_arg {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => exe_dir.join(path),
        None => exe_dir.join("runner_config.json"),
    }
}

fn redact_config_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, nested) in object.iter_mut() {
                let normalized = key.to_ascii_lowercase().replace('-', "_");
                if [
                    "password",
                    "passwd",
                    "token",
                    "secret",
                    "credential",
                    "authorization",
                    "api_key",
                    "apikey",
                ]
                .iter()
                .any(|needle| normalized.contains(needle))
                {
                    *nested = serde_json::Value::String("[REDACTED]".to_string());
                } else {
                    redact_config_value(nested);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                redact_config_value(item);
            }
        }
        _ => {}
    }
}

fn runner_config_log_snapshot(config: &crm_tool::runner::config::RunnerConfig) -> Result<String> {
    let mut value = serde_json::to_value(config)?;
    redact_config_value(&mut value);
    Ok(serde_json::to_string_pretty(&value)?)
}

#[tokio::main]
async fn main() -> Result<()> {
    if let InterceptResult::ExitSuccessfully = intercept_manifest(get_manifest()) {
        return Ok(());
    }
    let options = RunnerCliOptions::parse();
    let _instance_lock = match std::net::TcpListener::bind("127.0.0.1:14592") {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Runner already running (or port in use): {}", e);
            std::process::exit(0);
        }
    };

    let exe_dir = executable_dir().context("Failed to determine executable directory")?;
    let config_path = resolve_config_path(options.config.as_deref(), &exe_dir);
    let runner_config_path_str = config_path.to_string_lossy().to_string();
    let config_exists = config_path.exists();

    let runner_cfg = {
        let cfg = if config_exists {
            match crm_tool::runner::config::RunnerConfig::load(&runner_config_path_str) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "CRITICAL ERROR: Failed to parse existing runner config {}: {}",
                        config_path.display(),
                        e
                    );
                    std::process::exit(1);
                }
            }
        } else {
            crm_tool::runner::config::RunnerConfig::default()
        };
        if let Err(e) = cfg.validate() {
            eprintln!("Runner configuration validation failed: {}", e);
            std::process::exit(1);
        }
        if let Err(e) = crm_tool::runner::engine::validation::validate_config(&cfg) {
            eprintln!("Runner configuration logic validation failed: {}", e);
            std::process::exit(1);
        }
        cfg
    };

    let config_snapshot = runner_config_log_snapshot(&runner_cfg)?;
    let _log_guard = match setup_logging_with_levels_and_header(
        "runner",
        parse_log_level(&runner_cfg.log_stdout_level)?,
        parse_log_level(&runner_cfg.log_file_level)?,
        &config_snapshot,
    ) {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Failed to set up logging: {}", e);
            std::process::exit(1);
        }
    };

    info!("==================================================");
    info!("RUNNER - Starting tray scheduler mode");
    info!("==================================================");
    info!(
        "Loaded runner configuration successfully with {} tasks",
        runner_cfg.tasks.len()
    );

    let runner_handle = start_scheduler(runner_config_path_str.clone());
    start_gui_server(runner_handle.clone());

    let tx = runner_handle.command_tx.clone();
    if !config_exists {
        tokio::spawn(async move {
            let _ = tx.send(RunnerCommand::RunAllNow { is_manual: false }).await;
        });
    }

    #[cfg(target_os = "windows")]
    let event_loop = EventLoop::new()?;

    #[cfg(target_os = "windows")]
    let mut app = App {
        tray_icon: None,
        menu_items: None,
        runner: runner_handle,
        runner_gui_url: format!("http://{}:{}", runner_cfg.gui_host, runner_cfg.gui_port),
    };

    #[cfg(target_os = "windows")]
    {
        event_loop.run_app(&mut app)?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        info!("Headless mode: scheduler and GUI server are running. Press Ctrl+C to exit.");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C, initiating graceful shutdown...");
                let _ = runner_handle.command_tx.send(RunnerCommand::Shutdown).await;
                // Give it a moment to process the shutdown command and kill processes
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
        info!("Shutting down runner daemon.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runner_config_path_supports_default_relative_and_absolute_paths() {
        let exe_dir = Path::new("CRM");
        assert_eq!(
            resolve_config_path(None, exe_dir),
            exe_dir.join("runner_config.json")
        );
        assert_eq!(
            resolve_config_path(Some(Path::new("settings/custom.json")), exe_dir),
            exe_dir.join("settings/custom.json")
        );
        let temp = tempfile::tempdir().unwrap();
        let absolute = temp.path().join("runner.json");
        assert_eq!(resolve_config_path(Some(&absolute), exe_dir), absolute);
    }

    #[test]
    fn runner_config_log_snapshot_redacts_secret_fields() {
        let mut value = serde_json::json!({
            "api_token": "secret-value",
            "tasks": [{"args": {"--password": "pw", "--report": "tickets"}}]
        });
        redact_config_value(&mut value);
        assert_eq!(value["api_token"], "[REDACTED]");
        assert_eq!(value["tasks"][0]["args"]["--password"], "[REDACTED]");
        assert_eq!(value["tasks"][0]["args"]["--report"], "tickets");
    }
}

#[cfg(target_os = "windows")]
struct App {
    tray_icon: Option<TrayIcon>,
    menu_items: Option<(muda::MenuId, muda::MenuId, muda::MenuId, muda::MenuId)>,
    runner: RunnerHandle,
    runner_gui_url: String,
}

#[cfg(target_os = "windows")]
impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        if self.tray_icon.is_some() {
            return;
        }

        let menu = Menu::new();
        let run_now_i = MenuItem::new("Run All Tasks Now", true, None);
        let open_gui_i = MenuItem::new("Open Runner GUI", true, None);
        let logs_i = MenuItem::new("View Logs", true, None);
        let quit_i = MenuItem::new("Exit", true, None);

        let items: [&dyn IsMenuItem; 5] = [
            &run_now_i,
            &open_gui_i,
            &logs_i,
            &PredefinedMenuItem::separator(),
            &quit_i,
        ];

        if let Err(e) = menu.append_items(&items) {
            error!("Failed to build menu: {}", e);
        }

        match TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("CRM Runner")
            .with_icon(load_icon().unwrap_or_else(|e| {
                error!("Using fallback icon due to error: {}", e);
                // Return a dummy transparent/black icon rather than crashing the whole UI.
                Icon::from_rgba(vec![0; 4], 1, 1).unwrap()
            }))
            .build()
        {
            Ok(icon) => {
                self.tray_icon = Some(icon);
                self.menu_items = Some((
                    quit_i.id().clone(),
                    run_now_i.id().clone(),
                    logs_i.id().clone(),
                    open_gui_i.id().clone(),
                ));
                info!("Tray icon initialized.");
            }
            Err(e) => error!("Failed to build tray icon: {}", e),
        }
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: WindowId, _event: WindowEvent) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some((quit_id, run_id, logs_id, open_gui_id)) = &self.menu_items {
            if let Ok(event) = muda::MenuEvent::receiver().try_recv() {
                if event.id == *quit_id {
                    info!("Exit requested from menu.");
                    let tx = self.runner.command_tx.clone();
                    tokio::spawn(async move {
                        let _ = tx.send(RunnerCommand::Shutdown).await;
                    });
                    event_loop.exit();
                } else if event.id == *run_id {
                    info!("Manual run-all requested.");
                    let tx = self.runner.command_tx.clone();
                    tokio::spawn(async move {
                        let _ = tx.send(RunnerCommand::RunAllNow { is_manual: true }).await;
                    });
                } else if event.id == *logs_id {
                    info!("Opening logs file.");
                    if let Ok(exe_path) = std::env::current_exe() {
                        if let Some(exe_dir) = exe_path.parent() {
                            let log_path = exe_dir.join("runner.log");
                            let _ = open::that(log_path);
                        }
                    }
                } else if event.id == *open_gui_id {
                    info!("Opening runner GUI.");
                    let _ = open::that(&self.runner_gui_url);
                }
            }
        }

        let next_poll = std::time::Instant::now() + Duration::from_millis(200);
        event_loop.set_control_flow(ControlFlow::WaitUntil(next_poll));
    }
}

#[cfg(target_os = "windows")]
fn load_icon() -> anyhow::Result<Icon> {
    let width = 32;
    let height = 32;
    let rgba = [0, 255, 0, 255].repeat((width * height) as usize);
    Icon::from_rgba(rgba, width, height)
        .map_err(|e| anyhow::anyhow!("Failed to create icon: {:?}", e))
}
