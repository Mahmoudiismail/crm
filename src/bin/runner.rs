#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use anyhow::{Context, Result};
use crm_tool::runner::config::{ReportType, RunnerConfig};
use crm_tool::runner::engine::{start_scheduler, RunnerCommand, RunnerHandle};
use crm_tool::runner::gui::start_gui_server;
use muda::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{error, info};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

#[tokio::main]
async fn main() -> Result<()> {
    let _instance_lock = match std::net::TcpListener::bind("127.0.0.1:14592") {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Runner already running (or port in use): {}", e);
            std::process::exit(0);
        }
    };

    let _log_guard = match setup_logging() {
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
    let runner_cfg = RunnerConfig::load(&runner_config_path_str)?;
    ensure_crm_config_exists(&runner_cfg).await?;

    let runner_handle = start_scheduler(runner_config_path_str);
    start_gui_server(runner_handle.clone());

    let tx = runner_handle.command_tx.clone();
    if !config_exists {
        tokio::spawn(async move {
            let _ = tx.send(RunnerCommand::RunAllNow).await;
        });
    }

    #[cfg(target_os = "linux")]
    if let Err(e) = gtk::init() {
        error!("Failed to initialize GTK: {}", e);
    }
    let event_loop = EventLoop::new()?;
    let mut app = App {
        tray_icon: None,
        menu_items: None,
        runner: runner_handle,
        runner_gui_url: format!("http://{}:{}", runner_cfg.gui_host, runner_cfg.gui_port),
    };

    event_loop.run_app(&mut app)?;
    Ok(())
}

struct App {
    tray_icon: Option<TrayIcon>,
    menu_items: Option<(
        muda::MenuId,
        muda::MenuId,
        muda::MenuId,
        muda::MenuId,
        muda::MenuId,
    )>,
    runner: RunnerHandle,
    runner_gui_url: String,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        if self.tray_icon.is_some() {
            return;
        }

        let menu = Menu::new();
        let run_now_i = MenuItem::new("Run All Tasks Now", true, None);
        let run_tickets_i = MenuItem::new("Run CRM (Tickets Only)", true, None);
        let open_gui_i = MenuItem::new("Open Runner GUI", true, None);
        let logs_i = MenuItem::new("View Logs", true, None);
        let quit_i = MenuItem::new("Exit", true, None);

        let items: [&dyn IsMenuItem; 6] = [
            &run_now_i,
            &run_tickets_i,
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
                    run_tickets_i.id().clone(),
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
        if let Some((quit_id, run_id, run_tickets_id, logs_id, open_gui_id)) = &self.menu_items {
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
                } else if event.id == *run_tickets_id {
                    info!("Manual CRM tickets run requested.");
                    let tx = self.runner.command_tx.clone();
                    tokio::spawn(async move {
                        let _ = tx
                            .send(RunnerCommand::RunAdhocCrm(ReportType::Tickets))
                            .await;
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

fn load_icon() -> Icon {
    let width = 32;
    let height = 32;
    let rgba = [0, 255, 0, 255].repeat((width * height) as usize);
    Icon::from_rgba(rgba, width, height).unwrap_or_else(|_| panic!("Failed to create icon"))
}

fn setup_logging() -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let exe_dir = executable_dir();

    let file_appender = tracing_appender::rolling::never(exe_dir, "runner.log");
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_filter(LevelFilter::DEBUG);

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_thread_ids(false)
        .with_filter(LevelFilter::DEBUG);

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .init();

    Ok(guard)
}

fn executable_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

async fn ensure_crm_config_exists(cfg: &RunnerConfig) -> Result<()> {
    let config_path = resolve_relative_to_exe_dir(&cfg.crm_config_path);
    if Path::new(&config_path).exists() {
        return Ok(());
    }

    let crm_exec = resolve_crm_executable(&cfg.crm_executable_path);
    info!(
        "CRM config not found at {}. Initializing via {}",
        config_path,
        crm_exec.display()
    );

    let output = tokio::process::Command::new(&crm_exec)
        .arg("--config")
        .arg(&config_path)
        .arg("--report")
        .arg("none")
        .output()
        .await
        .with_context(|| format!("Failed to launch CRM executable: {}", crm_exec.display()))?;

    if !output.status.success() && !Path::new(&config_path).exists() {
        return Err(anyhow::anyhow!(
            "Failed to initialize CRM config using {} (status {:?}): {}",
            crm_exec.display(),
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

fn resolve_relative_to_exe_dir(path: &str) -> String {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        return p.to_string_lossy().to_string();
    }
    executable_dir().join(p).to_string_lossy().to_string()
}

fn resolve_crm_executable(configured: &str) -> PathBuf {
    let configured = configured.trim();
    let configured_name = if configured.is_empty() {
        default_crm_binary_name().to_string()
    } else {
        configured.to_string()
    };

    let candidate = PathBuf::from(&configured_name);
    if candidate.is_absolute() {
        return candidate;
    }

    let sibling = executable_dir().join(&configured_name);
    if sibling.exists() {
        return sibling;
    }

    candidate
}

fn default_crm_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "crm.exe"
    } else {
        "crm"
    }
}
