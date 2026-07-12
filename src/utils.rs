use anyhow::{Context, Result};
use crate::manifest::AppManifest;
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

pub fn executable_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn setup_logging(app_name: &str) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let log_dir = executable_dir();
    let file_appender = tracing_appender::rolling::never(&log_dir, format!("{}.log", app_name));
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG);

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_thread_ids(false)
        .with_filter(tracing_subscriber::filter::LevelFilter::INFO);

    let _ = tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .try_init();

    Ok(guard)
}

pub fn intercept_manifest(manifest: AppManifest) {
    for arg in std::env::args().skip(1) {
        if arg == "--manifest" {
            if let Ok(json) = serde_json::to_string(&manifest) {
                println!("{}", json);
            }
            std::process::exit(0);
        }
    }
}

pub fn load_or_create_config<T: DeserializeOwned + Serialize>(
    config_path: &Path,
    default_config: &T,
) -> Result<T> {
    if !config_path.exists() {
        let content = serde_json::to_string_pretty(default_config)
            .context("Failed to serialize default config")?;
        std::fs::write(config_path, content)
            .with_context(|| format!("Failed to write default config at {:?}", config_path))?;
    }

    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file at {:?}", config_path))?;

    let config: T = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file at {:?}", config_path))?;

    Ok(config)
}
