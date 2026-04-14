pub mod auth;
pub mod config;
pub mod fetcher;
pub mod downloader;
pub mod types;

use anyhow::Result;
use tracing::error;

use config::AppConfig;
use types::ReportType;

pub async fn run_once(
	crm_config_path: &str,
	report: ReportType,
) -> Result<()> {
	let mut config = AppConfig::load(crm_config_path)?;
	config.finalize_runtime_fields();

	let client = build_client(&config)?;

	let token = auth::ensure_authenticated(&mut config, &client, false).await?;
	config.save(crm_config_path)?;

	let results = fetcher::fetch_reports(&config, &client, &token, report).await?;

	if config.download_csv {
		let urls = fetcher::extract_urls(&results);
		let exe_path = std::env::current_exe()?;
		let exe_dir = exe_path
			.parent()
			.unwrap_or_else(|| std::path::Path::new("."));
		let download_dir = exe_dir.join("Downloads");
		std::fs::create_dir_all(&download_dir)?;

		for (key, url) in &urls {
			if let Err(e) = downloader::download_csv(&client, url, key, &download_dir).await {
				error!("Download failed for {}: {:#}", key, e);
			}
		}
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
