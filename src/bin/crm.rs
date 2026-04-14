use anyhow::Result;
use crm_tool::crm;
use crm_tool::crm::types::ReportType;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

const CRM_CONFIG_PATH: &str = "config.json";

#[tokio::main]
async fn main() -> Result<()> {
    let _log_guard = setup_logging()?;

    info!("==================================================");
    info!("CRM - One-shot run started");
    info!("==================================================");

    crm::run_once(CRM_CONFIG_PATH, ReportType::All, false, None).await?;

    info!("CRM - One-shot run completed successfully");
    Ok(())
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
