use anyhow::{Context, Result};
use headless_chrome::{protocol::cdp::types::Event, Browser, Tab};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;
use tracing::{info, warn};

// `Browser::new_tab` waits for a CDP target-created event. Serialize calls so
// concurrent report jobs cannot race while opening tabs.
static NEW_TAB_LOCK: Mutex<()> = Mutex::new(());

/// Opens an isolated browser tab for one report execution, retrying transient CDP failures.
///
/// # Examples
///
/// ```no_run
/// use headless_chrome::Browser;
/// use crm_tool::yasweb::browser::client::get_or_create_tab;
///
/// # fn example(browser: std::sync::Arc<Browser>) -> anyhow::Result<()> {
/// let tab = get_or_create_tab(&browser)?;
/// # Ok(())
/// # }
/// ```
///
/// [`Browser`]: https://docs.rs/chromiumoxide/latest/chromiumoxide/browser/struct.Browser.html
pub fn get_or_create_tab(browser: &Arc<Browser>) -> Result<Arc<Tab>> {
    let _guard = NEW_TAB_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut last_error = None;

    // Each report needs its own tab; sharing the first tab lets concurrent reports
    // navigate or manipulate the same page.
    for attempt in 1..=3 {
        match browser.new_tab() {
            Ok(tab) => return Ok(tab),
            Err(err) => {
                warn!("Failed to open isolated browser tab (attempt {}/3): {:#}", attempt, err);
                last_error = Some(err);
                if attempt < 3 {
                    std::thread::sleep(Duration::from_millis(250 * attempt));
                }
            }
        }
    }

    match last_error {
        Some(err) => Err(err).context("Failed to open a new isolated browser tab after 3 attempts"),
        None => Err(anyhow::anyhow!("Tab creation did not run")),
    }
}

pub fn enable_network_logging(
    tab: &Arc<Tab>,
) -> Result<Weak<dyn headless_chrome::browser::tab::EventListener<Event> + Send + Sync>> {
    tab.enable_log()
        .context("Failed to enable network domain")?;

    let events = tab
        .add_event_listener(Arc::new(|event: &Event| match event {
            Event::NetworkRequestWillBeSent(req) => {
                info!(
                    "Request: {} {}",
                    req.params.request.method,
                    redact_url(&req.params.request.url)
                );
            }
            Event::NetworkResponseReceived(res) => {
                let headers = res
                    .params
                    .response
                    .headers
                    .0
                    .as_ref()
                    .and_then(|headers| headers.as_object())
                    .into_iter()
                    .flat_map(|headers| headers.iter())
                    .map(|(name, value)| {
                        let lower_name = name.to_ascii_lowercase();
                        if matches!(
                            lower_name.as_str(),
                            "authorization" | "proxy-authorization" | "cookie" | "set-cookie"
                        ) {
                            format!("{name}: [REDACTED]")
                        } else {
                            format!(
                                "{name}: {:?}",
                                value.as_str().unwrap_or("[non-string value]")
                            )
                        }
                    })
                    .collect::<Vec<_>>();
                info!(
                    "Response: {} {} {} Headers: {:?}",
                    res.params.response.status,
                    redact_url(&res.params.response.url),
                    res.params.response.mime_type,
                    headers
                );
            }
            _ => {}
        }))
        .context("Failed to add event listener")?;

    Ok(events)
}

/// Removes query strings and fragments, which commonly contain session or signed URL data.
fn redact_url(url: &str) -> String {
    let end = url.find(['?', '#']).unwrap_or(url.len());
    url[..end].to_string()
}

#[cfg(test)]
mod network_logging_tests {
    use super::redact_url;

    #[test]
    fn redacts_query_strings_and_fragments() {
        assert_eq!(
            redact_url("https://example.test/report?token=secret#section"),
            "https://example.test/report"
        );
        assert_eq!(
            redact_url("https://example.test/report"),
            "https://example.test/report"
        );
    }
}
