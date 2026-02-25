mod auth;
mod cli;
mod config;
mod downloader;
mod fetcher;

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer,
};

// Re-export LevelFilter locally — no env-filter crate needed
use tracing_subscriber::filter::LevelFilter;

use cli::CliArgs;
use config::AppConfig;

#[tokio::main]
async fn main() {
    // Parse CLI first (so --config is available)
    let args = CliArgs::parse();

    // Set up logging
    if let Err(e) = setup_logging() {
        eprintln!("Failed to set up logging: {}", e);
        std::process::exit(1);
    }

    // Banner
    let start_time = Utc::now();
    info!("==================================================");
    info!("CRM TOOL - Starting");
    info!("Time: {}", start_time.to_rfc3339());
    info!("==================================================");

    // Run the main logic; on error, log + exit 1
    if let Err(e) = run(args).await {
        error!("Fatal error: {:#}", e);
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }

    let end_time = Utc::now();
    info!("==================================================");
    info!("CRM TOOL - Completed");
    info!("Time: {}", end_time.to_rfc3339());
    info!("==================================================");
}

async fn run(args: CliArgs) -> Result<()> {
    // ── Load & merge config ──
    let config_path = args.config.clone();
    let mut config = AppConfig::load(&config_path)?;
    config.apply_cli_overrides(&args);

    info!("Config loaded from {}", config_path);
    info!("Region: {}, Pool: {}", config.region, config.user_pool_id);
    info!("Fetching reports for {} to {}", config.from_date, config.to_date);

    // ── Build HTTP client ──
    let client = build_client(&config)?;

    // ── Authentication ──
    let token = auth::ensure_authenticated(&mut config, &client, args.skip_login).await?;
    info!("Token acquired (length: {})", token.len());

    // ── Save config (tokens may have been updated) ──
    config.save(&config_path)?;

    // ── Fetch reports ──
    let results = fetcher::fetch_reports(&config, &client, &token, args.report).await?;

    // ── CSV downloads ──
    if config.download_csv {
        let urls = fetcher::extract_urls(&results);
        if urls.is_empty() {
            info!("No CSV URLs found in report results");
        } else {
            info!("Found {} CSV URL(s) to download", urls.len());
            for (key, url) in &urls {
                match downloader::download_csv(&client, url, key).await {
                    Ok(filename) => info!("Downloaded: {}", filename),
                    Err(e) => error!("Failed to download CSV for {}: {:#}", key, e),
                }
            }
        }
    }

    // ── Output ──
    let pretty = serde_json::to_string_pretty(&results)?;
    if let Some(ref output_path) = args.output {
        std::fs::write(output_path, &pretty)?;
        info!("Report data written to {}", output_path);
    } else {
        println!();
        println!("==================================================");
        println!("REPORT DATA:");
        println!("==================================================");
        println!("{}", pretty);
    }

    // ── Final config save ──
    config.save(&config_path)?;

    Ok(())
}

fn build_client(config: &AppConfig) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();

    if config.no_verify_ssl {
        info!("TLS certificate verification DISABLED");
        builder = builder.danger_accept_invalid_certs(true);
    }

    let client = builder.build()?;
    Ok(client)
}

fn setup_logging() -> Result<()> {

    // File appender — DEBUG level, append mode
    let file_appender = tracing_appender::rolling::never(".", "crm_tool.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // We need to keep the guard alive for the duration of the program.
    // Leak it so it lives for 'static.
    std::mem::forget(_guard);

    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_filter(LevelFilter::DEBUG);

    // Stdout — INFO level
    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_thread_ids(false)
        .with_filter(LevelFilter::INFO);

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .init();

    Ok(())
}
