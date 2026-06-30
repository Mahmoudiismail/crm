use anyhow::Result;
use crm_tool::crm;
use crm_tool::crm::types::ReportType;
use crm_tool::manifest::{AppArg, AppManifest, ArgType};
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

#[tokio::main]
async fn main() -> Result<()> {
    // Intercept --manifest before anything else
    for arg in std::env::args().skip(1) {
        if arg == "--manifest" {
            print_manifest();
            std::process::exit(0);
        }
    }

    let _log_guard = setup_logging()?;

    let options = parse_args()?;
    let exe_dir = executable_dir();
    let config_path = resolve_config_path(options.config.as_deref(), &exe_dir);

    info!("==================================================");
    info!("CRM - One-shot run started");
    info!("==================================================");

    crm::run_once(&config_path, options.report).await?;

    info!("CRM - One-shot run completed successfully");
    Ok(())
}

#[derive(Default)]
struct CrmCliOptions {
    report: ReportType,
    config: Option<String>,
}

fn parse_args() -> Result<CrmCliOptions> {
    let mut options = CrmCliOptions {
        report: ReportType::All,
        ..Default::default()
    };

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--report" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("Missing value for --report"))?;
                options.report = parse_report(&value)?;
            }
            "--config" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("Missing value for --config"))?;
                options.config = Some(value);
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--manifest" => {
                // Handled earlier, but keep here just in case
                print_manifest();
                std::process::exit(0);
            }
            other => {
                return Err(anyhow::anyhow!(
                    "Unknown argument '{}'. Use --help to see supported options.",
                    other
                ));
            }
        }
    }

    Ok(options)
}

fn parse_report(value: &str) -> Result<ReportType> {
    match value.to_ascii_lowercase().as_str() {
        "all" => Ok(ReportType::All),
        "tickets" => Ok(ReportType::Tickets),
        "calls" => Ok(ReportType::Calls),
        "leads" => Ok(ReportType::Leads),
        "none" => Ok(ReportType::None),
        _ => Err(anyhow::anyhow!(
            "Invalid report '{}' (expected: all|tickets|calls|leads|none)",
            value
        )),
    }
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

fn print_manifest() {
    let manifest = AppManifest {
        name: "CRM One-Shot Fetcher".to_string(),
        description: "Fetches CRM data on a one-off basis.".to_string(),
        arguments: vec![
            AppArg {
                name: "--report".to_string(),
                arg_type: ArgType::List,
                required: false,
                default_value: Some("all".to_string()),
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
            },
        ],
    };
    if let Ok(json) = serde_json::to_string(&manifest) {
        println!("{}", json);
    }
}

fn executable_dir() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn print_help() {
    eprintln!("crm usage:\n  --report <all|tickets|calls|leads|none>\n  --config <path>");
}

fn setup_logging() -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    let file_appender = tracing_appender::rolling::never(exe_dir, "crm.log");
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
        .with_filter(LevelFilter::INFO);

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .init();

    Ok(guard)
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
