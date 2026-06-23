use anyhow::Result;
use crm_tool::crm;
use crm_tool::crm::types::ReportType;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

#[tokio::main]
async fn main() -> Result<()> {
    let _log_guard = setup_logging()?;

    let options = parse_args()?;
    let config_path = resolve_config_path(options.config.as_deref());

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

fn resolve_config_path(config_arg: Option<&str>) -> String {
    match config_arg {
        Some(path) => {
            let p = std::path::PathBuf::from(path);
            if p.is_absolute() {
                p.to_string_lossy().to_string()
            } else {
                executable_dir().join(p).to_string_lossy().to_string()
            }
        }
        None => executable_dir()
            .join("config.json")
            .to_string_lossy()
            .to_string(),
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

    #[test]
    fn test_parse_report_valid() {
        assert_eq!(parse_report("all").unwrap(), ReportType::All);
        assert_eq!(parse_report("tickets").unwrap(), ReportType::Tickets);
        assert_eq!(parse_report("calls").unwrap(), ReportType::Calls);
        assert_eq!(parse_report("leads").unwrap(), ReportType::Leads);
        assert_eq!(parse_report("none").unwrap(), ReportType::None);
    }

    #[test]
    fn test_parse_report_case_insensitive() {
        assert_eq!(parse_report("ALL").unwrap(), ReportType::All);
        assert_eq!(parse_report("TiCkEtS").unwrap(), ReportType::Tickets);
        assert_eq!(parse_report("CALLS").unwrap(), ReportType::Calls);
        assert_eq!(parse_report("lEaDs").unwrap(), ReportType::Leads);
        assert_eq!(parse_report("None").unwrap(), ReportType::None);
    }

    #[test]
    fn test_parse_report_invalid() {
        let err = parse_report("invalid").unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid report 'invalid' (expected: all|tickets|calls|leads|none)"
        );

        let err2 = parse_report("").unwrap_err();
        assert_eq!(
            err2.to_string(),
            "Invalid report '' (expected: all|tickets|calls|leads|none)"
        );
    }
}
