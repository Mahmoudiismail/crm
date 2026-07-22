#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use anyhow::Result;
use clap::Parser;
use crm_tool::manifest::AppManifest;
#[cfg(target_os = "windows")]
use crm_tool::runner::engine::RunnerHandle;
use crm_tool::runner::engine::{start_scheduler, RunnerCommand};
use crm_tool::runner::gui::start_gui_server;
use crm_tool::utils::{executable_dir, intercept_manifest, parse_log_level, setup_logging_with_levels};
#[cfg(target_os = "windows")]
use muda::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
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
    #[arg(long, hide = true)]
    manifest: bool,
}

fn get_manifest() -> AppManifest {
    AppManifest {
        name: "runner".to_string(),
        description: "Runner daemon".to_string(),
        arguments: vec![],
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    intercept_manifest(get_manifest());
    let _options = RunnerCliOptions::parse();
    let _instance_lock = match std::net::TcpListener::bind("127.0.0.1:14592") {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Runner already running (or port in use): {}", e);
            std::process::exit(0);
        }
    };

    let config_path = executable_dir().join("runner_config.json");
    let (stdout_lvl, file_lvl) = if config_path.exists() {
        if let Ok(raw) = std::fs::read_to_string(&config_path) {
            if let Ok(cfg) = serde_json::from_str::<crm_tool::runner::config::RunnerConfig>(&raw) {
                (cfg.log_stdout_level, cfg.log_file_level)
            } else {
                ("DEBUG".to_string(), "TRACE".to_string())
            }
        } else {
            ("DEBUG".to_string(), "TRACE".to_string())
        }
    } else {
        ("DEBUG".to_string(), "TRACE".to_string())
    };

    let _log_guard = match setup_logging_with_levels(
        "runner",
        parse_log_level(&stdout_lvl),
        parse_log_level(&file_lvl)
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

    let runner_config_path = executable_dir().join("runner_config.json");
    let runner_config_path_str = runner_config_path.to_string_lossy().to_string();

    let config_exists = runner_config_path.exists();

    let runner_handle = start_scheduler(runner_config_path_str.clone());
    start_gui_server(runner_handle.clone());

    let tx = runner_handle.command_tx.clone();
    if !config_exists {
        tokio::spawn(async move {
            let _ = tx.send(RunnerCommand::RunAllNow).await;
        });
    }

    #[cfg(target_os = "windows")]
    let event_loop = EventLoop::new()?;
    #[cfg(target_os = "windows")]
    let runner_cfg =
        crm_tool::runner::config::RunnerConfig::load(&runner_config_path_str).unwrap_or_default();

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
        info!("Headless mode: scheduler and GUI server are running.");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
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
            .with_icon(load_icon())
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
                    event_loop.exit();
                } else if event.id == *run_id {
                    info!("Manual run-all requested.");
                    let tx = self.runner.command_tx.clone();
                    tokio::spawn(async move {
                        let _ = tx.send(RunnerCommand::RunAllNow).await;
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
fn load_icon() -> Icon {
    let width = 32;
    let height = 32;
    let rgba = [0, 255, 0, 255].repeat((width * height) as usize);
    Icon::from_rgba(rgba, width, height).unwrap_or_else(|_| panic!("Failed to create icon"))
}
