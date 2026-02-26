#![windows_subsystem = "windows"]

mod core;
mod interface;
mod services;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use chrono_tz::Africa::Cairo;
use clap::Parser;
use muda::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

use tracing::{error, info};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

use crate::core::auth;
use crate::core::config::AppConfig;
use crate::interface::cli::CliArgs;
use crate::services::downloader;
use crate::services::fetcher;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI first
    let args = CliArgs::parse();

    // Set up logging
    if let Err(e) = setup_logging() {
        eprintln!("Failed to set up logging: {}", e);
        std::process::exit(1);
    }

    info!("==================================================");
    info!("CRM TOOL - Starting in Tray Mode");
    info!("==================================================");

    // Shared state
    let is_running = Arc::new(AtomicBool::new(false));

    // Spawn Scheduler Task
    let args_clone = args.clone();
    let is_running_clone = is_running.clone();
    tokio::spawn(async move {
        scheduler_loop(args_clone, is_running_clone).await;
    });

    // Run Fetcher Once at Startup
    let args_startup = args.clone();
    let is_running_startup = is_running.clone();
    tokio::spawn(async move {
        run_fetcher_safe(args_startup, is_running_startup).await;
    });

    // Run Event Loop
    let event_loop = EventLoop::new()?;
    let mut app = App {
        tray_icon: None,
        menu_items: None,
        args: args.clone(),
        is_running: is_running.clone(),
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}

struct App {
    tray_icon: Option<TrayIcon>,
    menu_items: Option<(muda::MenuId, muda::MenuId, muda::MenuId)>,
    args: CliArgs,
    is_running: Arc<AtomicBool>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        if self.tray_icon.is_some() {
            return;
        }

        // Initialize Tray Icon here
        let menu = Menu::new();
        let run_now_i = MenuItem::new("Run Fetcher Now", true, None);
        let logs_i = MenuItem::new("View Logs", true, None);
        let quit_i = MenuItem::new("Exit", true, None);

        let items: [&dyn IsMenuItem; 4] = [
            &run_now_i,
            &logs_i,
            &PredefinedMenuItem::separator(),
            &quit_i,
        ];

        if let Err(e) = menu.append_items(&items) {
            error!("Failed to build menu: {}", e);
        }

        match TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("CRM Tool")
            .with_icon(load_icon())
            .build()
        {
            Ok(icon) => {
                self.tray_icon = Some(icon);
                self.menu_items = Some((
                    quit_i.id().clone(),
                    run_now_i.id().clone(),
                    logs_i.id().clone(),
                ));
                info!("Tray icon initialized.");
            }
            Err(e) => error!("Failed to build tray icon: {}", e),
        }
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: WindowId, _event: WindowEvent) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Poll for events
        if let Some((quit_id, run_id, logs_id)) = &self.menu_items {
            if let Ok(event) = muda::MenuEvent::receiver().try_recv() {
                if &event.id == quit_id {
                    info!("Exit requested from menu.");
                    event_loop.exit();
                } else if &event.id == run_id {
                    info!("Manual run requested.");
                    let args = self.args.clone();
                    let is_running = self.is_running.clone();
                    tokio::spawn(async move {
                        run_fetcher_safe(args, is_running).await;
                    });
                } else if &event.id == logs_id {
                    info!("Opening logs.");
                    let _ = open::that("crm_tool.log");
                }
            }
        }

        // Use WaitUntil to poll periodically without busy loop
        // 200ms seems responsive enough
        let next_poll = std::time::Instant::now() + Duration::from_millis(200);
        event_loop.set_control_flow(ControlFlow::WaitUntil(next_poll));
    }
}

fn load_icon() -> Icon {
    let width = 32;
    let height = 32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..height {
        for _ in 0..width {
            // Green icon (0, 255, 0, 255)
            rgba.extend_from_slice(&[0, 255, 0, 255]);
        }
    }
    Icon::from_rgba(rgba, width, height).unwrap_or_else(|_| {
        // Fallback or panic? Icon creation usually succeeds for valid bytes
        panic!("Failed to create icon");
    })
}

async fn scheduler_loop(args: CliArgs, is_running: Arc<AtomicBool>) {
    info!("Scheduler started.");
    loop {
        // Reload config to catch changes to scheduled_time
        let config_path = args.config.clone();
        let config = match AppConfig::load(&config_path) {
            Ok(c) => c,
            Err(e) => {
                error!("Scheduler failed to load config: {}", e);
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }
        };

        let now_cairo = Utc::now().with_timezone(&Cairo);

        // Parse scheduled time (HH:MM)
        let scheduled_time =
            match chrono::NaiveTime::parse_from_str(&config.scheduled_time, "%H:%M") {
                Ok(t) => t,
                Err(_) => {
                    error!(
                        "Invalid scheduled_time format '{}', defaulting to 01:00",
                        config.scheduled_time
                    );
                    chrono::NaiveTime::from_hms_opt(1, 0, 0).unwrap()
                }
            };

        // Determine next run time
        let today_schedule = now_cairo
            .date_naive()
            .and_time(scheduled_time)
            .and_local_timezone(Cairo)
            .unwrap();

        let next_run = if today_schedule > now_cairo {
            today_schedule
        } else {
            today_schedule + chrono::Duration::days(1)
        };

        let duration = (next_run - now_cairo)
            .to_std()
            .unwrap_or(Duration::from_secs(60));
        info!("Next scheduled run at {} (in {:?})", next_run, duration);

        tokio::time::sleep(duration).await;

        info!("Scheduler waking up for scheduled run.");
        run_fetcher_safe(args.clone(), is_running.clone()).await;

        // Sleep a bit to avoid double trigger if clock drifts slightly backwards (unlikely with sleep)
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

async fn run_fetcher_safe(args: CliArgs, is_running: Arc<AtomicBool>) {
    // Attempt to set running flag
    if is_running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        info!("Fetcher is already running. Skipping this request.");
        return;
    }

    info!("Starting fetcher workflow...");
    if let Err(e) = run_once(args).await {
        error!("Fetcher workflow failed: {:#}", e);
    } else {
        info!("Fetcher workflow completed successfully.");
    }

    // Reset running flag
    is_running.store(false, Ordering::SeqCst);
}

/// The core logic (extracted from old main run function)
async fn run_once(args: CliArgs) -> Result<()> {
    // ── Load & merge config ──
    let config_path = args.config.clone();
    let mut config = AppConfig::load(&config_path)?;
    config.apply_cli_overrides(&args);

    info!("Config loaded from {}", config_path);
    info!("Region: {}, Pool: {}", config.region, config.user_pool_id);

    // Note: to_date is now defaulted to Egypt time in config.rs
    info!(
        "Fetching reports for {} to {}",
        config.from_date, config.to_date
    );

    // ── Build HTTP client ──
    let client = build_client(&config)?;

    // ── Authentication ──
    let token = auth::ensure_authenticated(&mut config, &client, args.skip_login).await?;
    info!("Token acquired (length: {})", token.len());

    // ── Save config (tokens may have been updated) ──
    config.save(&config_path)?;

    // ── Fetch reports ──
    let results = fetcher::fetch_reports(&config, &client, &token, args.report).await?;

    // ── CSV downloads ──
    if config.download_csv {
        let urls = fetcher::extract_urls(&results);
        if urls.is_empty() {
            info!("No CSV URLs found in report results");
        } else {
            // Determine download directory: <exe_dir>/download
            let exe_path = std::env::current_exe()?;
            let exe_dir = exe_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));
            let download_dir = exe_dir.join("download");

            info!(
                "Found {} CSV URL(s) to download. Target dir: {:?}",
                urls.len(),
                download_dir
            );
            for (key, url) in &urls {
                match downloader::download_csv(&client, url, key, &download_dir).await {
                    Ok(filename) => info!("Downloaded: {}", filename),
                    Err(e) => error!("Failed to download CSV for {}: {:#}", key, e),
                }
            }
        }
    }

    // ── Output (only log to file, no println since hidden window) ──
    // Or write to file if specified
    if let Some(ref output_path) = args.output {
        let pretty = serde_json::to_string_pretty(&results)?;
        std::fs::write(output_path, &pretty)?;
        info!("Report data written to {}", output_path);
    }

    // ── Final config save ──
    config.save(&config_path)?;

    Ok(())
}

fn build_client(config: &AppConfig) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();

    if config.no_verify_ssl {
        info!("TLS certificate verification DISABLED");
        builder = builder.danger_accept_invalid_certs(true);
    }

    let client = builder.build()?;
    Ok(client)
}

fn setup_logging() -> Result<()> {
    // File appender — DEBUG level
    let file_appender = tracing_appender::rolling::never(".", "crm_tool.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // We need to keep the guard alive.
    std::mem::forget(_guard);

    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_filter(LevelFilter::DEBUG);

    // Stdout — INFO level (still useful if run from console for debugging)
    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_thread_ids(false)
        .with_filter(LevelFilter::INFO);

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .init();

    Ok(())
}
