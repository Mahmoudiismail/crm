use std::sync::Arc;
use tracing::info;

/// HTML snapshots are disabled in production because authenticated pages may contain
/// credentials, session data, or customer information.
pub fn save_html_state(
    _tab: &Arc<headless_chrome::Tab>,
    _active_report_name: &str,
    _step_num: u32,
    step_name: &str,
) {
    info!(
        "Skipping HTML snapshot for '{}' because page contents may contain sensitive data",
        step_name
    );
}

/// Raw page diagnostics are intentionally not persisted. DOM text can contain sensitive
/// customer data even when form fields and URLs are redacted.
pub fn save_failure_diagnostics(_tab: &Arc<headless_chrome::Tab>, step_name: &str) {
    info!(
        "Skipping browser page diagnostics for '{}' to avoid persisting sensitive page contents",
        step_name
    );
}
