pub mod auth;
pub mod config;
pub mod downloader;
pub mod fetcher;

use anyhow::Result;

use config::AppConfig;

use anyhow::Context;

pub async fn run_once(
    config: &mut AppConfig,
    crm_config_path: &std::path::Path,
    report: &[String],
    start_date: Option<String>,
    end_date: Option<String>,
    custom_download_folder_cli: Option<String>,
) -> Result<()> {
    use crate::utils::to_iso_date_with_base;

    let resolved_start = if let Some(sd) = start_date {
        if !sd.is_empty() {
            let parsed = to_iso_date_with_base(&sd, None);
            config.from_date = parsed.clone();
            Some(parsed)
        } else {
            None
        }
    } else {
        None
    };

    if let Some(ed) = end_date {
        if !ed.is_empty() {
            config.to_date = to_iso_date_with_base(&ed, resolved_start.as_deref());
        }
    }

    config.finalize_runtime_fields();
    tracing::trace!(
        "Runtime fields finalized: from_date={}, to_date={}",
        config.from_date,
        config.to_date
    );

    // Validate that from_date <= to_date if both are present
    if !config.from_date.is_empty() && !config.to_date.is_empty() {
        if let (Some(start_dt), Some(end_dt)) = (
            crate::utils::parse_flexible_date(&config.from_date),
            crate::utils::parse_flexible_date(&config.to_date),
        ) {
            if start_dt > end_dt {
                anyhow::bail!(
                    "Validation failed: from_date ({}) is after to_date ({})",
                    config.from_date,
                    config.to_date
                );
            }
        }
    }

    let client = build_client(config).context("Failed to build HTTP client")?;

    tracing::info!("Ensuring authentication...");
    let _token = auth::ensure_authenticated(config, &client, false)
        .await
        .context("Failed during authentication process")?;
    tracing::trace!("Authentication successful.");
    let path_str = crm_config_path.to_string_lossy();
    config
        .save(&path_str)
        .context("Failed to save configuration after authentication")?;

    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;
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
        tokio::fs::create_dir_all(&download_dir)
            .await
            .with_context(|| format!("Failed to create download directory {:?}", download_dir))?;
    }

    tracing::info!("Fetching reports for type: {:?}", report);

    let config_arc = std::sync::Arc::new(tokio::sync::Mutex::new(config.clone()));

    let _results =
        fetcher::fetch_reports(config_arc.clone(), &client, report.to_vec(), &download_dir)
            .await
            .context("Failed to fetch CRM reports")?;
    tracing::trace!("Fetch reports results received.");

    {
        let final_cfg = config_arc.lock().await;
        *config = final_cfg.clone();
    }

    config
        .save(&path_str)
        .context("Failed to save configuration after fetching reports")?;
    Ok(())
}

fn build_client(config: &AppConfig) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if config.no_verify_ssl {
        builder = builder.danger_accept_invalid_certs(true);
    }
    Ok(builder.build()?)
}
