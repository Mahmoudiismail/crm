use anyhow::{Context, Result};
use crm_tool::tasker::config::{TaskConfig, TaskerConfig};
use crm_tool::tasker::csv_task;
use std::env;
use std::fs;

fn main() -> Result<()> {
    // Basic setup for logging/printing
    println!("Tasker started.");

    // Determine config path from args, default to "tasker_config.json"
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "tasker_config.json"
    };

    println!("Loading configuration from: {}", config_path);
    let config_content = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read tasker config at {}", config_path))?;

    let config: TaskerConfig = serde_json::from_str(&config_content)
        .with_context(|| "Failed to parse tasker_config.json")?;

    for (i, task) in config.tasks.iter().enumerate() {
        println!("Running task #{}", i + 1);
        match task {
            TaskConfig::CsvAnalysis(csv_config) => {
                if let Err(e) = csv_task::run(csv_config) {
                    eprintln!("Error running CsvAnalysis task: {:?}", e);
                }
            }
        }
    }

    println!("All tasks completed.");
    Ok(())
}
