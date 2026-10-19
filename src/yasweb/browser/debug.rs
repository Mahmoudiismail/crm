use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info};

pub fn save_html_state(
    tab: &Arc<headless_chrome::Tab>,
    active_report_name: &str,
    step_num: u32,
    step_name: &str,
) {
    let get_html_js = r#"
        (function() {
            let mainHtml = document.documentElement ? document.documentElement.outerHTML : "";
            let iframeHtml = "";
            try {
                let iframe = document.querySelector('iframe');
                if (iframe && iframe.contentWindow && iframe.contentWindow.document && iframe.contentWindow.document.documentElement) {
                    iframeHtml = iframe.contentWindow.document.documentElement.outerHTML;
                }
            } catch (e) {
                iframeHtml = "ERROR ACCESSING IFRAME: " + e.message;
            }
            return "=== MAIN DOCUMENT ===\n" + mainHtml + "\n\n=== IFRAME DOCUMENT ===\n" + iframeHtml;
        })();
    "#;

    match tab.evaluate(get_html_js, true) {
        Ok(res) => {
            let Some(value) = res.value else {
                error!("HTML extraction at '{}' returned no value", step_name);
                return;
            };
            let Some(html) = value.as_str() else {
                error!("HTML extraction at '{}' returned a non-string value", step_name);
                return;
            };

            let mut exe_dir = match crate::utils::executable_dir() {
                Ok(dir) => dir,
                Err(e) => {
                    error!("Cannot resolve executable directory for HTML snapshot '{}': {e}", step_name);
                    return;
                }
            };
            exe_dir.push("debug_html");
            if let Err(e) = std::fs::create_dir_all(&exe_dir) {
                error!("Cannot create HTML snapshot directory {:?}: {e}", exe_dir);
                return;
            }

            let captured_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_micros())
                .unwrap_or_default();
            let safe_name =
                active_report_name.replace(|c: char| !c.is_alphanumeric(), "_");
            let safe_step = step_name
                .replace(|c: char| !c.is_alphanumeric(), "_")
                .to_lowercase();
            let file_name = format!(
                "step_{:02}_{}_{}_{}.html",
                step_num, captured_at, safe_step, safe_name
            );

            exe_dir.push(&file_name);
            match std::fs::write(&exe_dir, html) {
                Ok(()) => info!("Saved HTML state for '{}' to {:?}", step_name, exe_dir),
                Err(e) => error!("Failed to write HTML state for '{}' to {:?}: {e}", step_name, exe_dir),
            }
        }
        Err(e) => {
            error!("Failed to extract HTML at {}: {:?}", step_name, e);
        }
    }
}

/// Saves a redacted page snapshot and browser metadata when an automation step fails.
/// The diagnostic file is kept separate from routine step snapshots to make failures easy to find.
pub fn save_failure_diagnostics(tab: &Arc<headless_chrome::Tab>, step_name: &str) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let diagnostic_js = r##"
        (() => {
            const safeUrl = (value) => {
                try {
                    const u = new URL(value, location.href);
                    u.search = "";
                    u.hash = "";
                    return u.href;
                } catch (_) { return String(value || "").split(/[?#]/, 1)[0]; }
            };
            const redactDocument = (doc) => {
                if (!doc || !doc.documentElement) return "";
                const clone = doc.documentElement.cloneNode(true);
                for (const el of clone.querySelectorAll("input, textarea, select")) {
                    const type = (el.getAttribute("type") || "").toLowerCase();
                    const name = (el.getAttribute("name") || "") + " " + (el.getAttribute("id") || "");
                    if (type === "password" || /pass|token|secret|auth|cookie/i.test(name)) {
                        el.removeAttribute("value");
                        el.textContent = "";
                    } else if (el.tagName === "TEXTAREA") {
                        el.textContent = "[REDACTED]";
                    } else {
                        el.removeAttribute("value");
                    }
                }
                for (const el of clone.querySelectorAll("[value]")) {
                    const name = (el.getAttribute("name") || "") + " " + (el.getAttribute("id") || "");
                    if (/pass|token|secret|auth|cookie/i.test(name)) el.setAttribute("value", "[REDACTED]");
                }
                return clone.outerHTML;
            };
            const frameSnapshots = Array.from(document.querySelectorAll("iframe")).map((frame, index) => {
                const metadata = {
                    index,
                    src: safeUrl(frame.getAttribute("src") || ""),
                    title: frame.title || "",
                    loaded: !!frame.contentDocument,
                    accessible: false
                };
                try {
                    const frameDoc = frame.contentWindow && frame.contentWindow.document;
                    metadata.accessible = !!(frameDoc && frameDoc.documentElement);
                    return { ...metadata, html: metadata.accessible ? redactDocument(frameDoc) : "" };
                } catch (e) {
                    return { ...metadata, error: String(e && e.message || e), html: "" };
                }
            });
            const selectorDiagnostics = {};
            for (const selector of [
                "#menuPinnedBtn", "#pinButton", ".menuModules", ".misManagement",
                "input[formcontrolname='searchInput']", "input[placeholder='Search']",
                ".tree-view", "#loader_svg", ".loading-screen-wrapper",
                "mat-progress-bar", ".dx-loadpanel"
            ]) {
                selectorDiagnostics[selector] = Array.from(document.querySelectorAll(selector)).slice(0, 100).map(el => {
                    const style = el.ownerDocument.defaultView.getComputedStyle(el);
                    return {
                        tag: el.tagName,
                        id: el.id || "",
                        className: typeof el.className === "string" ? el.className : "",
                        text: String(el.innerText || el.textContent || "").trim().slice(0, 500),
                        visible: style.display !== "none" && style.visibility !== "hidden" && Number(style.opacity || 1) > 0,
                        disabled: !!el.disabled
                    };
                });
            }
            const active = document.activeElement;
            const firstFrame = frameSnapshots.length ? frameSnapshots[0] : null;
            const navigation = performance.getEntriesByType("navigation")[0];
            return JSON.stringify({
                capturedAt: new Date().toISOString(),
                url: safeUrl(location.href),
                title: document.title,
                readyState: document.readyState,
                visibilityState: document.visibilityState,
                hasFocus: document.hasFocus(),
                online: navigator.onLine,
                documentHtmlLength: document.documentElement ? document.documentElement.outerHTML.length : 0,
                activeElement: active ? {
                    tag: active.tagName,
                    id: active.id || "",
                    name: active.getAttribute("name") || "",
                    type: active.getAttribute("type") || "",
                    className: typeof active.className === "string" ? active.className : ""
                } : null,
                selectorDiagnostics,
                frames: frameSnapshots.map(({ html, ...metadata }) => metadata),
                frameSnapshots,
                mainHtml: redactDocument(document),
                iframeHtml: firstFrame ? (firstFrame.html || firstFrame.error || "") : "",
                navigationTiming: navigation ? {
                    durationMs: navigation.duration,
                    domInteractiveMs: navigation.domInteractive,
                    domContentLoadedMs: navigation.domContentLoadedEventEnd,
                    loadEventMs: navigation.loadEventEnd
                } : null,
                resourceCount: performance.getEntriesByType("resource").length
            });
        })()
    "##;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let safe_step = step_name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_lowercase();

    let mut path = match crate::utils::executable_dir() {
        Ok(dir) => dir,
        Err(e) => {
            error!("Cannot resolve executable directory for failure diagnostics: {e}");
            return;
        }
    };
    path.push("debug_html");
    if let Err(e) = std::fs::create_dir_all(&path) {
        error!(
            "Cannot create failure diagnostics directory {:?}: {e}",
            path
        );
        return;
    }
    path.push(format!("failure_{now}_{safe_step}.json"));

    match tab.evaluate(diagnostic_js, true) {
        Ok(result) => match result
            .value
            .and_then(|value| value.as_str().map(str::to_owned))
        {
            Some(json) => match std::fs::write(&path, json) {
                Ok(()) => error!(
                    "Saved redacted failure diagnostics for '{}' to {:?}",
                    step_name, path
                ),
                Err(e) => error!(
                    "Failed to write failure diagnostics for '{}': {e}",
                    step_name
                ),
            },
            None => error!(
                "Failure diagnostics for '{}' returned no string result",
                step_name
            ),
        },
        Err(e) => error!(
            "Failed to collect failure diagnostics for '{}': {e}",
            step_name
        ),
    }
}
