pub mod auth;
pub mod config;
pub mod downloader;
pub mod fetcher;
pub mod types;

use anyhow::Result;
use futures_util::future::join_all;
use tracing::error;

use config::AppConfig;
use types::ReportType;

pub async fn run_once(
    crm_config_path: &str,
    report: ReportType,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<()> {
    let mut config = AppConfig::load(crm_config_path)?;

    use crate::utils::to_iso_date;

    if let Some(sd) = start_date {
        if !sd.is_empty() {
            config.from_date = to_iso_date(&sd);
        }
    }
    if let Some(ed) = end_date {
        if !ed.is_empty() {
            config.to_date = to_iso_date(&ed);
        }
    }

    config.finalize_runtime_fields();
    tracing::trace!("Runtime fields finalized: from_date={}, to_date={}", config.from_date, config.to_date);

    let client = build_client(&config)?;

    tracing::info!("Ensuring authentication...");
    let token = auth::ensure_authenticated(&mut config, &client, false).await?;
    tracing::trace!("Authentication successful.");
    config.save(crm_config_path)?;

    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let download_dir = exe_dir.join("Downloads");

    tracing::info!("Fetching reports for type: {:?}", report);
    let results = fetcher::fetch_reports(&config, &client, &token, report, &download_dir).await?;
    tracing::trace!("Fetch reports results received.");

    if config.download_csv {
        let urls = fetcher::extract_urls(&results);
        tracing::info!("Extracted {} download URL(s).", urls.len());
        tokio::fs::create_dir_all(&download_dir).await?;

        let download_futures = urls.iter().map(|(key, url)| {
            let client = client.clone();
            let download_dir = download_dir.clone();
            async move {
                if let Err(e) = downloader::download_csv(&client, url, key, &download_dir).await {
                    error!("Download failed for {}: {:#}", key, e);
                }
            }
        });
        join_all(download_futures).await;
    }

    config.save(crm_config_path)?;
    Ok(())
}

fn build_client(config: &AppConfig) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if config.no_verify_ssl {
        builder = builder.danger_accept_invalid_certs(true);
    }
    Ok(builder.build()?)
}
