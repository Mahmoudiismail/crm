use headless_chrome::Tab;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Persists page HTML only when the existing TRACE log level is enabled.
/// Saved pages can contain session data and customer information.
pub fn save_html_state(
    tab: &Arc<Tab>,
    active_report_name: &str,
    step_num: u32,
    step_name: &str,
) {
    if !tracing::enabled!(tracing::Level::TRACE) {
        debug!(
            "Skipping HTML snapshot for '{}' because TRACE logging is disabled",
            step_name
        );
        return;
    }

    persist_html(tab, active_report_name, Some(step_num), step_name);
}

/// Saves the current page after a browser-stage failure when TRACE is enabled.
pub fn save_failure_diagnostics(tab: &Arc<Tab>, step_name: &str) {
    if !tracing::enabled!(tracing::Level::TRACE) {
        debug!(
            "Skipping browser page diagnostics for '{}' because TRACE logging is disabled",
            step_name
        );
        return;
    }

    persist_html(tab, "failure", None, step_name);
}

fn persist_html(
    tab: &Arc<Tab>,
    report_name: &str,
    step_num: Option<u32>,
    step_name: &str,
) {
    let html = match tab.get_content() {
        Ok(html) => html,
        Err(error) => {
            warn!(
                "Failed to capture HTML for browser step '{}': {}",
                step_name, error
            );
            return;
        }
    };

    let exe_dir = match crate::utils::executable_dir() {
        Ok(path) => path,
        Err(error) => {
            warn!(
                "Failed to locate executable directory for HTML snapshot '{}': {}",
                step_name, error
            );
            return;
        }
    };

    let debug_dir = exe_dir.join("debug_html");
    if let Err(error) = std::fs::create_dir_all(&debug_dir) {
        warn!(
            "Failed to create HTML snapshot directory {:?}: {}",
            debug_dir, error
        );
        return;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let step = step_num
        .map(|number| format!("{number:02}_{}", sanitize_filename(step_name)))
        .unwrap_or_else(|| sanitize_filename(step_name));
    let filename = format!(
        "{}_{}_{}_{}.html",
        sanitize_filename(report_name),
        step,
        std::process::id(),
        timestamp
    );
    let path: PathBuf = debug_dir.join(filename);

    match std::fs::write(&path, html) {
        Ok(()) => info!("Saved browser HTML snapshot: {:?}", path),
        Err(error) => warn!("Failed to save browser HTML snapshot {:?}: {}", path, error),
    }
}

fn sanitize_filename(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "page".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_filename;

    #[test]
    fn snapshot_filename_components_cannot_escape_debug_directory() {
        assert_eq!(sanitize_filename("../../report\page"), "______report_page");
        assert_eq!(sanitize_filename("After login"), "After_login");
        assert_eq!(sanitize_filename(""), "page");
    }
}
