use anyhow::{Context, Result};
use headless_chrome::{protocol::cdp::types::Event, Browser, Tab};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tracing::info;

/// Retrieves the first available browser tab or opens a new one when no tab exists.
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
    let mut found = None;
    for _ in 0..10 {
        let tabs = browser.get_tabs().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(first) = tabs.first() {
            found = Some(first.clone());
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    match found {
        Some(t) => Ok(t),
        None => browser.new_tab().context("Failed to open new tab"),
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
                    req.params.request.method, req.params.request.url
                );
            }
            Event::NetworkResponseReceived(res) => {
                info!(
                    "Response: {} {} {} Headers: {:?}",
                    res.params.response.status,
                    res.params.response.url,
                    res.params.response.mime_type,
                    res.params.response.headers
                );
            }
            _ => {}
        }))
        .context("Failed to add event listener")?;

    Ok(events)
}
