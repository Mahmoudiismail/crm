pub mod client;
pub mod debug;
pub mod download;
pub mod javascript;
pub mod login;
pub mod reports;

use anyhow::Result;
use headless_chrome::Browser;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::yasweb::config::YaswebConfig;

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
) -> Result<Vec<String>> {
    let mut step_num = 1;
    let tab = client::get_or_create_tab(&browser)?;

    // Configure download behavior
    download::configure_download_directory(&tab, download_dir.as_ref());

    // Setup network listeners
    let events = client::enable_network_logging(&tab)?;

    // Perform Login
    if let Err(e) = login::execute_login(&tab, config, active_report_name, &mut step_num) {
        if config.keep_open {
            std::thread::sleep(Duration::from_secs(60));
        }
        return Err(e);
    }

    // Run report extraction
    let discovered_filters = match reports::navigate_and_run_report(
        &tab,
        active_report_name,
        active_report_type,
        active_filters,
        config.timeout_minutes,
        &mut step_num,
    ) {
        Ok(filters) => filters,
        Err(e) => {
            if config.keep_open {
                std::thread::sleep(Duration::from_secs(60));
            }
            return Err(e);
        }
    };

    // Wait for download if applicable
    download::wait_for_download(download_dir.as_ref(), config.timeout_minutes);

    // Clean up listeners
    if let Err(e) = tab.remove_event_listener(&events) {
        tracing::error!("Failed to remove listener: {:?}", e);
    }

    if config.keep_open {
        std::thread::sleep(Duration::from_secs(60));
    }

    Ok(discovered_filters)
}
