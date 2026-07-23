use anyhow::Result;
use clap::Parser;
use crm_tool::crm;
use crm_tool::manifest::{AppArg, AppManifest, ArgType};
use crm_tool::utils::{
    executable_dir, intercept_manifest, parse_log_level, setup_logging_with_levels,
};

use tracing::info;

#[derive(Parser)]
#[command(name = "crm", about = "CRM One-Shot Fetcher")]
struct CrmCliOptions {
    #[arg(long, value_delimiter = ',', default_value = "tickets")]
    report: Vec<String>,
    #[arg(long)]
    config: Option<String>,
    #[arg(long)]
    start_date: Option<String>,
    #[arg(long)]
    end_date: Option<String>,
    #[arg(long)]
    custom_download_folder: Option<String>,
    #[arg(long, hide = true)]
    manifest: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    intercept_manifest(get_manifest());

    let options = CrmCliOptions::parse();
    let exe_dir = executable_dir();
    let config_path = resolve_config_path(options.config.as_deref(), &exe_dir);

    // Load config to grab logging levels early
    let early_config = crm_tool::crm::config::AppConfig::load(&config_path)?;

    let _log_guard = setup_logging_with_levels(
        "crm",
        parse_log_level(&early_config.log_stdout_level),
        parse_log_level(&early_config.log_file_level),
    )?;

    info!("==================================================");
    info!("CRM - One-shot run started");
    info!("==================================================");

    use crm_tool::utils::replace_date_vars;

    let start_date = options.start_date.map(|s| replace_date_vars(&s, None));
    let end_date = options
        .end_date
        .map(|e| replace_date_vars(&e, start_date.as_deref()));

    crm::run_once(
        &config_path,
        options.report,
        start_date,
        end_date,
        options.custom_download_folder,
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
            AppArg::new("--report", ArgType::MultiList)
                .default_value("tickets")
                .options(vec!["all", "tickets", "calls", "leads", "users", "none"]),
            AppArg::new("--config", ArgType::String),
            AppArg::new("--start-date", ArgType::DateVar),
            AppArg::new("--end-date", ArgType::DateVar),
            AppArg::new("--custom-download-folder", ArgType::String),
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
