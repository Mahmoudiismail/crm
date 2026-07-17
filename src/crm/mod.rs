pub mod auth;
pub mod config;
pub mod downloader;
pub mod fetcher;

use anyhow::Result;
use tracing::error;

use config::AppConfig;

pub async fn run_once(
    crm_config_path: &str,
    report: Vec<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    custom_download_folder_cli: Option<String>,
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
    tracing::trace!(
        "Runtime fields finalized: from_date={}, to_date={}",
        config.from_date,
        config.to_date
    );

    let client = build_client(&config)?;

    tracing::info!("Ensuring authentication...");
    let token = auth::ensure_authenticated(&mut config, &client, false).await?;
    tracing::trace!("Authentication successful.");
    config.save(crm_config_path)?;

    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    let custom_dl = custom_download_folder_cli
        .filter(|s| !s.trim().is_empty())
        .or_else(|| config.custom_download_folder.clone());

    let download_dir = if let Some(ref custom_path) = custom_dl {
        let p = std::path::PathBuf::from(custom_path);
        let target = if p.is_absolute() { p } else { exe_dir.join(p) };
        tracing::info!("Using custom download folder: {:?}", target);
        target
    } else {
        let target = exe_dir.join("Downloads");
        tracing::info!("Using default download folder: {:?}", target);
        target
    };

    // Ensure download dir exists upfront if needed
    if config.download_csv {
        if let Err(e) = tokio::fs::create_dir_all(&download_dir).await {
            error!(
                "Failed to create download directory {:?}: {:#}",
                download_dir, e
            );
        }
    }

    tracing::info!("Fetching reports for type: {:?}", report);
    let _results = fetcher::fetch_reports(&config, &client, &token, report, &download_dir).await?;
    tracing::trace!("Fetch reports results received.");

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
