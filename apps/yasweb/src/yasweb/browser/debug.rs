use base64::Engine;
use headless_chrome::{protocol::cdp::Page, Tab};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Persists page HTML only when the existing TRACE log level is enabled.
/// Saved pages can contain session data and customer information.
pub fn save_html_state(tab: &Arc<Tab>, active_report_name: &str, step_num: u32, step_name: &str) {
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

fn persist_html(tab: &Arc<Tab>, report_name: &str, step_num: Option<u32>, step_name: &str) {
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

    persist_frame_resources(tab, &debug_dir, report_name, &step, timestamp);
}

/// Captures each document frame through CDP. Unlike `Tab::get_content`, this can
/// retrieve cross-origin iframe documents without relying on parent-page JS access.
fn persist_frame_resources(
    tab: &Arc<Tab>,
    debug_dir: &std::path::Path,
    report_name: &str,
    step: &str,
    timestamp: u128,
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
                warn!("Invalid CDP frame id for '{}': {}", frame_url, error);
                continue;
            }
        };

        match tab.call_method(Page::GetResourceContent {
            frame_id: frame_id_value,
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

                let filename = format!(
                    "{}_{}_frame{}_{}_{}.html",
                    sanitize_filename(report_name),
                    step,
                    index,
                    std::process::id(),
                    timestamp
                );
                let path = debug_dir.join(filename);
                match std::fs::write(&path, bytes) {
                    Ok(()) => info!(
                        "Saved browser frame HTML: frame_id={} url={} path={:?}",
                        frame_id, frame_url, path
                    ),
                    Err(error) => warn!(
                        "Failed to save browser frame HTML for {}: {}",
                        frame_url, error
                    ),
                }
            }
            Err(error) => warn!(
                "Could not capture frame HTML: frame_id={} url={} error={:#}",
                frame_id, frame_url, error
            ),
        }
    }
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
    use super::{collect_frame_resources, sanitize_filename};

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
    fn snapshot_filename_components_cannot_escape_debug_directory() {
        assert_eq!(sanitize_filename("../../report\\page"), "______report_page");
        assert_eq!(sanitize_filename("After login"), "After_login");
        assert_eq!(sanitize_filename(""), "page");
    }
}
