use std::sync::Arc;
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
            if let Some(v) = res.value {
                if let Some(html) = v.as_str() {
                    if let Ok(mut exe_dir) = crate::utils::executable_dir() {
                        exe_dir.push("debug_html");
                        let _ = std::fs::create_dir_all(&exe_dir);

                        let safe_name =
                            active_report_name.replace(|c: char| !c.is_alphanumeric(), "_");
                        let safe_step = step_name
                            .replace(|c: char| !c.is_alphanumeric(), "_")
                            .to_lowercase();
                        let file_name =
                            format!("step_{:02}_{}_{}.html", step_num, safe_step, safe_name);

                        exe_dir.push(&file_name);
                        if std::fs::write(&exe_dir, html).is_ok() {
                            info!("Saved HTML state for '{}' to {:?}", step_name, exe_dir);
                        } else {
                            error!("Failed to write HTML state for '{}' to disk", step_name);
                        }
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to extract HTML at {}: {:?}", step_name, e);
        }
    }
}
