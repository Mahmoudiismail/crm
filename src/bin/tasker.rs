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

    // Determine config path from args, default to "tasker_config.json"
    // And ensure the tasker looks for the configuration file in the same directory as the executable
    let args: Vec<String> = env::args().collect();

    let default_config_path = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("tasker_config.json")))
        .unwrap_or_else(|| std::path::PathBuf::from("tasker_config.json"));

    let config_path = if args.len() > 1 {
        std::path::PathBuf::from(&args[1])
    } else {
        default_config_path
    };

    info!("Loading configuration from: {}", config_path.display());
    let config_content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read tasker config at {}", config_path.display()))?;

    let config: TaskerConfig = serde_json::from_str(&config_content)
        .with_context(|| "Failed to parse tasker_config.json")?;

    for (i, task) in config.tasks.iter().enumerate() {
        info!("Running task #{}", i + 1);
        match task {
            TaskConfig::CsvAnalysis(csv_config) => {
                if let Err(e) = csv_task::run(csv_config) {
                    error!("Error running CsvAnalysis task: {:?}", e);
                }
            }
        }
    }

    info!("All tasks completed.");
    Ok(())
}
