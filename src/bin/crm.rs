use anyhow::Result;
use clap::Parser;
use crm_tool::crm;
use crm_tool::crm::types::ReportType;
use crm_tool::manifest::{AppArg, AppManifest, ArgType};
use crm_tool::utils::{executable_dir, intercept_manifest, setup_logging};

use tracing::info;

#[derive(Parser)]
#[command(name = "crm", about = "CRM One-Shot Fetcher")]
struct CrmCliOptions {
    #[arg(long, value_enum, default_value_t = ReportType::All)]
    report: ReportType,
    #[arg(long)]
    config: Option<String>,
    #[arg(long)]
    start_date: Option<String>,
    #[arg(long)]
    end_date: Option<String>,
    #[arg(long, hide = true)]
    manifest: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    intercept_manifest(get_manifest());

    let _log_guard = setup_logging("crm")?;

    let options = CrmCliOptions::parse();
    let exe_dir = executable_dir();
    let config_path = resolve_config_path(options.config.as_deref(), &exe_dir);

    info!("==================================================");
    info!("CRM - One-shot run started");
    info!("==================================================");

    crm::run_once(
        &config_path,
        options.report,
        options.start_date,
        options.end_date,
    )
    .await?;

    info!("CRM - One-shot run completed successfully");
    Ok(())
}

fn resolve_config_path(config_arg: Option<&str>, base_dir: &std::path::Path) -> String {
    match config_arg {
        Some(path) => {
            let p = std::path::PathBuf::from(path);
            if p.is_absolute() {
                p.to_string_lossy().to_string()
            } else {
                base_dir.join(p).to_string_lossy().to_string()
            }
        }
        None => base_dir.join("config.json").to_string_lossy().to_string(),
    }
}

fn get_manifest() -> AppManifest {
    AppManifest {
        name: "CRM One-Shot Fetcher".to_string(),
        description: "Fetches CRM data on a one-off basis.".to_string(),
        arguments: vec![
            AppArg {
                name: "--report".to_string(),
                arg_type: ArgType::List,
                required: false,
                default_value: Some("all".to_string()),
                depends_on: None,
                autofill: None,
                options: Some(vec![
                    "all".to_string(),
                    "tickets".to_string(),
                    "calls".to_string(),
                    "leads".to_string(),
                    "none".to_string(),
                ]),
            },
            AppArg {
                name: "--config".to_string(),
                arg_type: ArgType::String,
                required: false,
                default_value: None,
                options: None,
                depends_on: None,
                autofill: None,
            },
            AppArg {
                name: "--start-date".to_string(),
                arg_type: ArgType::String,
                required: false,
                default_value: None,
                options: None,
                depends_on: None,
                autofill: None,
            },
            AppArg {
                name: "--end-date".to_string(),
                arg_type: ArgType::String,
                required: false,
                default_value: None,
                options: None,
                depends_on: None,
                autofill: None,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_resolve_config_path_none() {
        let expected = executable_dir()
            .join("config.json")
            .to_string_lossy()
            .to_string();
        assert_eq!(resolve_config_path(None, &executable_dir()), expected);
    }

    #[test]
    fn test_resolve_config_path_some_relative() {
        let relative_path = "custom_config.json";
        let expected = executable_dir()
            .join(relative_path)
            .to_string_lossy()
            .to_string();
        assert_eq!(
            resolve_config_path(Some(relative_path), &executable_dir()),
            expected
        );
    }

    #[test]
    fn test_resolve_config_path_some_absolute() {
        // Use an OS-appropriate absolute path
        let absolute_path = if cfg!(windows) {
            "C:\\foo\\bar\\config.json"
        } else {
            "/foo/bar/config.json"
        };

        let expected = PathBuf::from(absolute_path).to_string_lossy().to_string();
        assert_eq!(
            resolve_config_path(Some(absolute_path), &executable_dir()),
            expected
        );
    }
}
