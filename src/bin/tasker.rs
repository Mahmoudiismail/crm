use anyhow::{Context, Result};
use crm_tool::tasker::config::{TaskConfig, TaskerConfig};
use crm_tool::tasker::csv_task;
use std::env;
use std::fs;
use tracing::{error, info};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<()> {
    // Setup file logging in the same directory as the executable
    let log_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let file_appender = RollingFileAppender::new(Rotation::NEVER, log_dir, "task_csv_analysis.log");

    // We can also have a stdout logger if desired, but we'll stick to a simple combined setup.
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(file_appender))
        .with(fmt::layer().with_writer(std::io::stdout))
        .init();

    info!("Tasker started.");

    // Parse arguments
    let args: Vec<String> = env::args().collect();
    let mut config_path_arg = None;
    let mut task_filter: Option<usize> = None;
    let mut only_call_center = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                if i + 1 < args.len() {
                    config_path_arg = Some(std::path::PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--task" => {
                if i + 1 < args.len() {
                    if let Ok(idx) = args[i + 1].parse::<usize>() {
                        task_filter = Some(idx);
                    }
                    i += 1;
                }
            }
            "--only-call-center" => {
                only_call_center = true;
            }
            // Support the legacy positional config path if they don't provide a flag
            val if !val.starts_with("--") && config_path_arg.is_none() => {
                config_path_arg = Some(std::path::PathBuf::from(val));
            }
            _ => {}
        }
        i += 1;
    }

    let default_config_path = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("tasker_config.json")))
        .unwrap_or_else(|| std::path::PathBuf::from("tasker_config.json"));

    let config_path = config_path_arg.unwrap_or(default_config_path);

    info!("Loading configuration from: {}", config_path.display());
    let config_content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read tasker config at {}", config_path.display()))?;

    let config: TaskerConfig = serde_json::from_str(&config_content)
        .with_context(|| "Failed to parse tasker_config.json")?;

    for (i, task) in config.tasks.iter().enumerate() {
        let task_idx = i + 1;

        if let Some(filter) = task_filter {
            if task_idx != filter {
                continue;
            }
        }

        info!("Running task #{}", task_idx);
        match task {
            TaskConfig::CsvAnalysis(csv_config) => {
                if let Err(e) = csv_task::run(csv_config, only_call_center) {
                    error!("Error running CsvAnalysis task: {:?}", e);
                }
            }
        }
    }

    info!("All tasks completed.");
    Ok(())
}
