pub mod client;
pub mod debug;
pub mod download;
pub mod javascript;
pub mod login;
pub mod reports;

use anyhow::{Context, Result};
use headless_chrome::Browser;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info};

use crate::yasweb::config::{DiscoveredReportFilters, YaswebConfig};

// Public API Re-exports matching original monolith
pub use debug::save_html_state;
pub use download::get_global_download_dir;

pub fn run_browser_tab(
    browser: Arc<Browser>,
    config: &YaswebConfig,
    active_report_name: &str,
    active_report_type: &str,
    active_filters: &HashMap<String, String>,
    download_dir: Option<PathBuf>,
) -> Result<DiscoveredReportFilters> {
    let task_started = Instant::now();
    info!(
        "YASWEB RUN START: report='{}' type='{}' keep_open={} headless={} timeout_minutes={}",
        active_report_name,
        active_report_type,
        config.keep_open,
        config.headless,
        config.timeout_minutes
    );
    let mut step_num = 1;
    let tab =
        client::get_or_create_tab(&browser).context("Yasweb stage failed: create browser tab")?;
    info!("YASWEB STAGE START: configure_downloads");
    download::configure_download_directory(&tab, download_dir.as_ref())
        .context("Yasweb stage failed: configure download directory")?;

    info!("YASWEB STAGE START: enable_network_logging");
    let events = client::enable_network_logging(&tab)
        .context("Yasweb stage failed: enable network logging")?;

    info!("YASWEB STAGE START: login");
    let login_started = Instant::now();
    if let Err(e) = login::execute_login(&tab, config, active_report_name, &mut step_num) {
        error!(
            "YASWEB STAGE FAILED: login elapsed_ms={} error={:#}",
            login_started.elapsed().as_millis(),
            e
        );
        debug::save_failure_diagnostics(&tab, "login");
        if config.keep_open {
            info!("Keeping browser open for 60 seconds after login failure because keep_open=true");
            std::thread::sleep(Duration::from_secs(60));
        }
        return Err(e.context("Yasweb stage failed: login"));
    }
    info!(
        "YASWEB STAGE END: login status=SUCCESS elapsed_ms={}",
        login_started.elapsed().as_millis()
    );

    // Snapshot existing files so a stale report cannot satisfy this export's wait.
    let download_snapshot = download::snapshot_downloads(download_dir.as_ref())
        .context("Yasweb stage failed: snapshot download directory")?;

    // Run report extraction
    info!("YASWEB STAGE START: report_navigation_and_export");
    let report_started = Instant::now();
    let (discovered_filters, download_already_completed) = match reports::navigate_and_run_report(
        &tab,
        active_report_name,
        active_report_type,
        active_filters,
        config.timeout_minutes,
        &mut step_num,
    ) {
        Ok(filters) => (filters, false),
        Err(report_error) => {
            error!(
                "YASWEB STAGE FAILED: report_navigation_and_export elapsed_ms={} error={:#}",
                report_started.elapsed().as_millis(),
                report_error
            );
            // Export can finish even when a later SPA navigation/readiness step fails.
            // Recover only when this run produced a new or changed stable report file.
            match download::wait_for_download_with_timeout(
                download_dir.as_ref(),
                Duration::from_secs(20),
                &download_snapshot,
            ) {
                Ok(()) => {
                    tracing::warn!(
                        "YASWEB recovered a completed report download after a report navigation/export error: report='{}'",
                        active_report_name
                    );
                    (DiscoveredReportFilters::default(), true)
                }
                Err(download_error) => {
                    error!(
                        "YASWEB download recovery failed after report error: {:#}",
                        download_error
                    );
                    debug::save_failure_diagnostics(&tab, "report_navigation_and_export");
                    if config.keep_open {
                        info!("Keeping browser open for 60 seconds after report failure because keep_open=true");
                        std::thread::sleep(Duration::from_secs(60));
                    }
                    return Err(report_error.context("Yasweb stage failed: report navigation/export"));
                }
            }
        }
    };
    info!(
        "YASWEB STAGE END: report_navigation_and_export status={} elapsed_ms={} discovered_filters={}",
        if download_already_completed { "RECOVERED_DOWNLOAD" } else { "SUCCESS" },
        report_started.elapsed().as_millis(),
        discovered_filters.names.len()
    );

    // Wait for download unless the bounded recovery wait already confirmed it.
    if !download_already_completed {
        info!("YASWEB STAGE START: wait_for_download");
        let download_started = Instant::now();
        if let Err(e) = download::wait_for_download(
            download_dir.as_ref(),
            config.timeout_minutes,
            &download_snapshot,
        ) {
            error!(
                "YASWEB STAGE FAILED: wait_for_download elapsed_ms={} error={:#}",
                download_started.elapsed().as_millis(),
                e
            );
            debug::save_failure_diagnostics(&tab, "wait_for_download");
            if config.keep_open {
                info!(
                    "Keeping browser open for 60 seconds after download failure because keep_open=true"
                );
                std::thread::sleep(Duration::from_secs(60));
            }
            return Err(e.context("Yasweb stage failed: wait for download"));
        }
        info!(
            "YASWEB STAGE END: wait_for_download status=SUCCESS elapsed_ms={}",
            download_started.elapsed().as_millis()
        );
    }
    info!(
        "YASWEB STAGE END: wait_for_download status=SUCCESS elapsed_ms={}",
        download_started.elapsed().as_millis()
    );

    // Clean up listeners
    if let Err(e) = tab.remove_event_listener(&events) {
        tracing::error!("Failed to remove listener: {:?}", e);
    }

    if config.keep_open {
        info!("Keeping browser open for 60 seconds after successful run because keep_open=true");
        std::thread::sleep(Duration::from_secs(60));
    }

    info!(
        "YASWEB RUN END: status=SUCCESS report='{}' elapsed_ms={}",
        active_report_name,
        task_started.elapsed().as_millis()
    );
    Ok(discovered_filters)
}
