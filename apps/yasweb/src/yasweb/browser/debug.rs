use base64::Engine;
use headless_chrome::{
    protocol::cdp::{Page, Runtime},
    Tab,
};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::{debug, info, warn};

static DIAGNOSTIC_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static HTML_DIAGNOSTICS_ENABLED: AtomicBool = AtomicBool::new(false);
static SAVED_SNAPSHOT_FINGERPRINTS: OnceLock<Mutex<HashSet<u64>>> = OnceLock::new();

/// Returns true only once for identical HTML belonging to the same report/document.
/// Failure snapshots bypass this check so the final broken state is always retained.
fn is_new_snapshot(report: &str, document: &str, html: &[u8]) -> bool {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    report.hash(&mut hasher);
    document.hash(&mut hasher);
    html.hash(&mut hasher);
    let fingerprint = hasher.finish();
    let fingerprints = SAVED_SNAPSHOT_FINGERPRINTS.get_or_init(|| Mutex::new(HashSet::new()));
    fingerprints
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(fingerprint)
}

/// Enables or disables persistence of authenticated page and iframe HTML.
pub fn set_html_diagnostics_enabled(enabled: bool) {
    HTML_DIAGNOSTICS_ENABLED.store(enabled, Ordering::Relaxed);
}

fn html_diagnostics_enabled() -> bool {
    HTML_DIAGNOSTICS_ENABLED.load(Ordering::Relaxed)
}

/// Persists page HTML only when the explicit save_html_diagnostics config option is enabled.
/// Saved pages can contain session data and customer information.
pub fn save_html_state(tab: &Arc<Tab>, active_report_name: &str, step_num: u32, step_name: &str) {
    if !html_diagnostics_enabled() {
        debug!(
            "Skipping HTML snapshot for '{}' because save_html_diagnostics is false",
            step_name
        );
        return;
    }

    persist_html(tab, active_report_name, Some(step_num), step_name, false);
}

/// Saves the current page after a browser-stage failure when diagnostics are explicitly enabled.
pub fn save_failure_diagnostics(tab: &Arc<Tab>, step_name: &str) {
    if !html_diagnostics_enabled() {
        debug!(
            "Skipping browser page diagnostics for '{}' because save_html_diagnostics is false",
            step_name
        );
        return;
    }

    persist_html(tab, "failure", None, step_name, true);
}

fn persist_html(tab: &Arc<Tab>, report_name: &str, step_num: Option<u32>, step_name: &str, force: bool) {
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

    if !force && !is_new_snapshot(report_name, "main-document", html.as_bytes()) {
        debug!("Skipping duplicate HTML snapshot: report='{}' step='{}'", report_name, step_name);
        return;
    }

    let debug_dir = exe_dir.join("debug_html");
    if let Err(error) = std::fs::create_dir_all(&debug_dir) {
        warn!(
            "Failed to create HTML snapshot directory {:?}: {}",
            debug_dir, error
        );
        return;
    }

    let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis(),
        Err(error) => {
            warn!(
                "System clock is before UNIX_EPOCH; skipping HTML snapshot '{}': {}",
                step_name, error
            );
            return;
        }
    };
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
    match write_diagnostic_file(&debug_dir, &filename, html.as_bytes()) {
        Ok(path) => info!("Saved browser HTML snapshot: {:?}", path),
        Err(error) => warn!(
            "Failed to save browser HTML snapshot '{}': {:#}",
            filename, error
        ),
    }

    persist_frame_resources(tab, &debug_dir, report_name, &step, timestamp, force);
}

/// Captures each document frame through CDP. Unlike `Tab::get_content`, this can
/// retrieve cross-origin iframe documents without relying on parent-page JS access.
fn persist_frame_resources(
    tab: &Arc<Tab>,
    debug_dir: &std::path::Path,
    report_name: &str,
    step: &str,
    timestamp: u128,
    force: bool,
) {
    let tree = match tab.call_method(Page::GetResourceTree(None)) {
        Ok(tree) => tree,
        Err(error) => {
            warn!(
                "Failed to inspect browser frame tree for '{}': {:#}",
                step, error
            );
            return;
        }
    };

    let tree_value = match serde_json::to_value(tree) {
        Ok(value) => value,
        Err(error) => {
            warn!(
                "Failed to serialize browser frame tree for '{}': {}",
                step, error
            );
            return;
        }
    };

    let mut frames = Vec::new();
    collect_frame_resources(&tree_value, &mut frames);
    info!(
        "Browser frame diagnostics: step='{}' frames={}",
        step,
        frames.len()
    );

    for (index, frame_id, frame_url) in frames {
        let frame_id_value = match serde_json::from_value::<Page::FrameId>(
            serde_json::Value::String(frame_id.clone()),
        ) {
            Ok(value) => value,
            Err(error) => {
                warn!(
                    "Invalid CDP frame id for '{}': {}",
                    redact_url_for_log(&frame_url),
                    error
                );
                continue;
            }
        };

        match tab.call_method(Page::GetResourceContent {
            frame_id: frame_id_value.clone(),
            url: frame_url.clone(),
        }) {
            Ok(resource) => {
                let bytes = if resource.base_64_encoded {
                    match base64::engine::general_purpose::STANDARD.decode(&resource.content) {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            warn!(
                                "Could not decode frame {} resource content: {}",
                                frame_id, error
                            );
                            continue;
                        }
                    }
                } else {
                    resource.content.into_bytes()
                };

                if !force && !is_new_snapshot(report_name, &frame_url, &bytes) {
                    debug!("Skipping duplicate frame HTML snapshot: report='{}' frame_url='{}'", report_name, redact_url_for_log(&frame_url));
                    continue;
                }

                let filename = format!(
                    "{}_{}_frame{}_{}_{}.html",
                    sanitize_filename(report_name),
                    step,
                    index,
                    std::process::id(),
                    timestamp
                );
                match write_diagnostic_file(debug_dir, &filename, &bytes) {
                    Ok(path) => info!(
                        "Saved browser frame HTML: frame_id={} url={} path={:?}",
                        frame_id,
                        redact_url_for_log(&frame_url),
                        path
                    ),
                    Err(error) => warn!(
                        "Failed to save browser frame HTML for {}: {:#}",
                        redact_url_for_log(&frame_url),
                        error
                    ),
                }
            }
            Err(error) => {
                warn!(
                    "Could not capture frame resource HTML; trying live frame DOM: frame_id={} url={} error={:#}",
                    frame_id, redact_url_for_log(&frame_url), error
                );
                let capture = (|| -> anyhow::Result<String> {
                    let world = tab.call_method(Page::CreateIsolatedWorld {
                        frame_id: frame_id_value.clone(),
                        world_name: Some("crm-yasweb-frame-diagnostics".to_string()),
                        grant_univeral_access: Some(false),
                    })?;
                    let response = tab.call_method(Runtime::Evaluate {
                        expression:
                            "document.documentElement ? document.documentElement.outerHTML : ''"
                                .to_string(),
                        object_group: None,
                        include_command_line_api: None,
                        silent: Some(true),
                        context_id: Some(world.execution_context_id),
                        return_by_value: Some(true),
                        generate_preview: None,
                        user_gesture: None,
                        await_promise: Some(false),
                        throw_on_side_effect: None,
                        timeout: None,
                        disable_breaks: None,
                        repl_mode: None,
                        allow_unsafe_eval_blocked_by_csp: None,
                        unique_context_id: None,
                        serialization_options: None,
                    })?;
                    response
                        .result
                        .value
                        .and_then(|value| value.as_str().map(str::to_owned))
                        .filter(|html| !html.is_empty())
                        .ok_or_else(|| anyhow::anyhow!("frame DOM evaluation returned no HTML"))
                })();
                match capture {
                    Ok(html) => {
                        if !force && !is_new_snapshot(report_name, &frame_url, html.as_bytes()) {
                            debug!("Skipping duplicate live frame DOM snapshot: report='{}' frame_url='{}'", report_name, redact_url_for_log(&frame_url));
                            continue;
                        }
                        let filename = format!(
                            "{}_{}_frame{}_{}_{}.html",
                            sanitize_filename(report_name),
                            step,
                            index,
                            std::process::id(),
                            timestamp
                        );
                        match write_diagnostic_file(debug_dir, &filename, html.as_bytes()) {
                            Ok(path) => info!(
                                "Saved live browser frame DOM: frame_id={} url={} path={:?}",
                                frame_id, redact_url_for_log(&frame_url), path
                            ),
                            Err(write_error) => warn!(
                                "Failed to save live frame DOM for {}: {:#}",
                                redact_url_for_log(&frame_url), write_error
                            ),
                        }
                    }
                    Err(capture_error) => warn!(
                        "Could not capture frame HTML through live DOM fallback: frame_id={} url={} error={:#}",
                        frame_id, redact_url_for_log(&frame_url), capture_error
                    ),
                }
            }
        }
    }
}

/// Writes a diagnostic snapshot to a unique temporary file and atomically renames it.
/// The final and temporary files share a directory so rename remains atomic.
fn write_diagnostic_file(
    directory: &std::path::Path,
    filename: &str,
    contents: &[u8],
) -> anyhow::Result<PathBuf> {
    let sequence = DIAGNOSTIC_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let requested = std::path::Path::new(filename);
    let stem = requested
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("snapshot");
    let extension = requested
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("html");
    let unique_name = format!("{stem}_{sequence}.{extension}");
    let final_path = directory.join(&unique_name);
    let temporary_path = directory.join(format!(".{stem}_{sequence}.{extension}.tmp"));

    std::fs::write(&temporary_path, contents).map_err(|error| {
        anyhow::anyhow!(
            "failed to write temporary diagnostic file {:?}: {}",
            temporary_path,
            error
        )
    })?;
    if let Err(error) = std::fs::rename(&temporary_path, &final_path) {
        let _ = std::fs::remove_file(&temporary_path);
        return Err(anyhow::anyhow!(
            "failed to atomically publish diagnostic file {:?}: {}",
            final_path,
            error
        ));
    }
    Ok(final_path)
}

/// Removes query strings and fragments from URLs before they enter diagnostic logs.
fn redact_url_for_log(url: &str) -> String {
    url.split(['?', '#']).next().unwrap_or(url).to_string()
}

fn collect_frame_resources(value: &serde_json::Value, frames: &mut Vec<(usize, String, String)>) {
    if let Some(frame_tree) = value.get("frameTree") {
        collect_frame_resources(frame_tree, frames);
        return;
    }
    if let Some(frame) = value.get("frame") {
        let frame_id = frame.get("id").and_then(serde_json::Value::as_str);
        let frame_url = frame.get("url").and_then(serde_json::Value::as_str);
        if let (Some(frame_id), Some(frame_url)) = (frame_id, frame_url) {
            if !frame_url.is_empty() {
                frames.push((frames.len(), frame_id.to_string(), frame_url.to_string()));
            }
        }
    }
    if let Some(children) = value
        .get("childFrames")
        .and_then(serde_json::Value::as_array)
    {
        for child in children {
            collect_frame_resources(child, frames);
        }
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
    use super::{
        collect_frame_resources, redact_url_for_log, sanitize_filename, write_diagnostic_file,
    };

    #[test]
    fn frame_tree_collection_includes_nested_cross_origin_frames() {
        let tree = serde_json::json!({
            "frameTree": {
                "frame": {"id": "root", "url": "https://portal.example/"},
                "childFrames": [{
                    "frame": {"id": "report", "url": "https://reports.example/report"},
                    "childFrames": [{
                        "frame": {"id": "nested", "url": "https://reports.example/lookup"}
                    }]
                }]
            }
        });
        let mut frames = Vec::new();
        collect_frame_resources(&tree, &mut frames);

        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].1, "root");
        assert_eq!(frames[1].1, "report");
        assert_eq!(frames[2].1, "nested");
    }

    #[test]
    fn diagnostic_snapshots_are_published_atomically_with_unique_names() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let first = write_diagnostic_file(directory.path(), "report_step_1.html", b"first")
            .expect("first diagnostic should be written");
        let second = write_diagnostic_file(directory.path(), "report_step_1.html", b"second")
            .expect("second diagnostic should be written");

        assert_ne!(first, second);
        assert_eq!(std::fs::read(first).expect("read first"), b"first");
        assert_eq!(std::fs::read(second).expect("read second"), b"second");
        assert_eq!(
            std::fs::read_dir(directory.path())
                .expect("read directory")
                .count(),
            2
        );
    }

    #[test]
    fn diagnostic_url_logs_drop_query_strings_and_fragments() {
        assert_eq!(
            redact_url_for_log("https://reports.example/report?session=secret#tab"),
            "https://reports.example/report"
        );
        assert_eq!(
            redact_url_for_log("https://reports.example/report"),
            "https://reports.example/report"
        );
    }

    #[test]
    fn snapshot_filename_components_cannot_escape_debug_directory() {
        assert_eq!(sanitize_filename("../../report\\page"), "______report_page");
        assert_eq!(sanitize_filename("After login"), "After_login");
        assert_eq!(sanitize_filename(""), "page");
    }
}
