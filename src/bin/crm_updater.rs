use anyhow::Result;
use clap::Parser;
use crm_tool::manifest::{AppArg, AppManifest, ArgType};
use crm_tool::utils::{intercept_manifest, InterceptResult};
use crm_tool::utils::{load_or_create_config, parse_log_level, setup_logging_with_levels};
use std::path::PathBuf;
use tracing::info;

use crm_tool::crm_updater::config::{ReplacementMapEntry, UpdaterConfig};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, help = "Only run the update pipeline")]
    update_only: bool,

    #[arg(long, help = "Only run the log rotation and sending pipeline")]
    logs_only: bool,

    #[arg(long, help = "Print the manifest and exit")]
    manifest: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.manifest {
        let manifest = AppManifest {
            name: "CRM Updater".to_string(),
            description: "Updates the CRM tool and runner, and rotates logs".to_string(),
            arguments: vec![
                AppArg::new("--update-only", ArgType::Boolean),
                AppArg::new("--logs-only", ArgType::Boolean),
            ],
        };

        if let InterceptResult::ExitSuccessfully = intercept_manifest(manifest) {
            return Ok(());
        }
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let config_path = exe_dir.join("updater_config.json");

    let default_config = UpdaterConfig {
        downloads_dir: "downloads".to_string(),
        runner_logs_dir: "logs".to_string(),
        log_recipient_email: "admin@example.com".to_string(),
        file_replacement_map: vec![
            ReplacementMapEntry {
                source_file: "crm_updater.exe".to_string(),
                target_path: ".".to_string(),
                executable_name: "crm_updater.exe".to_string(),
                restart_args: None,
                autostart: true,
            },
            ReplacementMapEntry {
                source_file: "runner.exe".to_string(),
                target_path: ".".to_string(),
                executable_name: "runner.exe".to_string(),
                restart_args: None,
                autostart: true,
            },
        ],
        log_stdout_level: "DEBUG".to_string(),
        log_file_level: "TRACE".to_string(),
    };

    if !config_path.exists() {
        load_or_create_config(&config_path, &default_config)?;
        println!(
            "Created default configuration file at {:?}. Please edit it and re-run.",
            config_path
        );
        return Ok(());
    }

    let config: UpdaterConfig = load_or_create_config(&config_path, &default_config)?;

    let _guard = setup_logging_with_levels(
        "crm_updater",
        parse_log_level(&config.log_stdout_level)?,
        parse_log_level(&config.log_file_level)?,
    )?;

    info!("Loaded config: {:#?}", config);

    let run_all = !args.update_only && !args.logs_only;

    if run_all || args.logs_only {
        if let Err(e) = crm_tool::crm_updater::logs::process_and_send_logs(&config) {
            tracing::error!("Logs pipeline failed: {}", e);
            return Err(e);
        }
    }

    if run_all || args.update_only {
        if let Err(e) = crm_tool::crm_updater::update::process_update_pipeline(&config) {
            tracing::error!("Update pipeline failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
