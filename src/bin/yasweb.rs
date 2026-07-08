use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate};
use crm_tool::manifest::{AppArg, AppManifest, ArgType};
use headless_chrome::protocol::cdp::types::Event;
use headless_chrome::{Browser, LaunchOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ReportConfig {
    report_type: String,
    #[serde(default)]
    filters: HashMap<String, String>,
    #[serde(default)]
    start_date_key: Option<String>,
    #[serde(default)]
    end_date_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct YaswebConfig {
    url: String,
    username: String,
    password: Option<String>,
    #[serde(default)]
    headless: bool,
    #[serde(default)]
    keep_open: bool,
    #[serde(default)]
    reports: HashMap<String, ReportConfig>,
}

impl Default for YaswebConfig {
    fn default() -> Self {
        Self {
            url: "https://yasweb.fakeeh.care:8030/".to_string(),
            username: "3245".to_string(),
            password: Some("Soso@2350181".to_string()),
            headless: false,
            keep_open: false,
            reports: HashMap::new(),
        }
    }
}

async fn load_or_create_config(path: &PathBuf) -> Result<YaswebConfig> {
    if path.exists() {
        let content = fs::read_to_string(path)
            .await
            .context("Failed to read yasweb_config.json")?;
        let config: YaswebConfig =
            serde_json::from_str(&content).context("Failed to parse yasweb_config.json")?;
        Ok(config)
    } else {
        let config = YaswebConfig::default();
        let content = serde_json::to_string_pretty(&config)
            .context("Failed to serialize default yasweb config")?;
        fs::write(path, content)
            .await
            .context("Failed to write default yasweb_config.json")?;
        Ok(config)
    }
}

fn setup_logging() -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let mut log_dir = std::env::current_exe().context("Failed to get executable path")?;
    log_dir.pop();
    let file_appender = tracing_appender::rolling::never(log_dir, "yasweblog");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_writer(non_blocking)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("Setting default subscriber failed")?;
    Ok(guard)
}

fn run_browser_tab(
    browser: Arc<Browser>,
    config: &YaswebConfig,
    active_report_name: &str,
    active_report_type: &str,
    active_filters: &HashMap<String, String>,
    is_initial_tab: bool,
    download_dir: Option<PathBuf>,
) -> Result<Vec<String>> {
    let mut discovered_filters = Vec::new();

    let tab = if is_initial_tab {
        let blank_tab = {
            let tabs = browser.get_tabs().lock().unwrap_or_else(|e| e.into_inner());
            let mut found = None;
            for t in tabs.iter() {
                let url = t.get_url();
                if url.contains("about:blank") || url.is_empty() {
                    found = Some(t.clone());
                    break;
                }
            }

            if found.is_none() {
                let urls: Vec<String> = tabs.iter().map(|t| t.get_url()).collect();
                info!("Warning: No about:blank tab found. Open tabs: {:?}", urls);
                found = tabs.first().cloned();
            }
            found
        };

        match blank_tab {
            Some(t) => t,
            None => browser.new_tab().context("Failed to open new tab")?,
        }
    } else {
        browser.new_tab().context("Failed to open new tab")?
    };

    // Configure download behavior to use temp dir
    if let Some(ref dl_dir) = download_dir {
        info!("Configuring download directory to {:?}", dl_dir);
        if let Err(e) = tab.call_method(headless_chrome::protocol::cdp::Page::SetDownloadBehavior {
            behavior:
                headless_chrome::protocol::cdp::Page::SetDownloadBehaviorBehaviorOption::Allow,
            download_path: Some(dl_dir.to_string_lossy().to_string()),
        }) {
            error!("Failed to set download behavior for tab: {:?}", e);
        }
    }

    // Enable network logging
    tab.enable_log()
        .context("Failed to enable network domain")?;

    // Add event listener for network
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
                    "Response: {} {} {}",
                    res.params.response.status,
                    res.params.response.url,
                    res.params.response.mime_type
                );
            }
            _ => {}
        }))
        .context("Failed to add event listener")?;

    info!("Navigating to {}", config.url);
    if let Err(e) = tab.navigate_to(&config.url) {
        error!("Navigate failed: {:?}", e);
        println!(
            "Warning: navigate to {} returned error, continuing anyway...",
            config.url
        );
    } else {
        info!("Successfully navigated to {}", config.url);
    }

    // Attempt to wait until navigated, ignore error if it timeouts but page loads
    let _ = tab.wait_until_navigated();

    // Give Angular more time to bootstrap and render the initial DOM
    std::thread::sleep(Duration::from_secs(10));

    info!("Waiting for username input...");
    let username_selector = "input[formcontrolname='username'], #mat-input-0";

    // Custom wait loop to wait longer than default timeout
    let mut username_found = false;
    for _ in 0..6 {
        // 6 * 5 = 30 seconds max wait
        if tab.wait_for_element(username_selector).is_ok() {
            username_found = true;
            break;
        }
        std::thread::sleep(Duration::from_secs(5));
    }

    if !username_found {
        error!("Failed to find username input after extended wait.");
        if let Ok(html) = tab.get_content() {
            error!("Page HTML at failure to find username:\n{}", html);
        }

        if config.keep_open {
            std::thread::sleep(Duration::from_secs(60));
        }
        return Err(anyhow::anyhow!("Failed to find elements to login"));
    }

    match tab.wait_for_element(username_selector) {
        Ok(user_input) => {
            info!("Typing username...");
            if let Err(e) = user_input.type_into(&config.username) {
                error!("Failed to type username: {:?}", e);
                if let Ok(html) = tab.get_content() {
                    error!("Page HTML at failure to type username:\n{}", html);
                }
                if config.keep_open {
                    std::thread::sleep(Duration::from_secs(60));
                }
                return Err(anyhow::anyhow!("Failed to type username"));
            }
            info!("Successfully typed username.");

            // Wait a brief moment to ensure page loads data after username
            std::thread::sleep(Duration::from_secs(2));

            if let Some(password) = &config.password {
                info!("Waiting for password input...");
                let password_selector = "input[formcontrolname='password'], #passFocus";
                match tab.wait_for_element(password_selector) {
                    Ok(pass_input) => {
                        info!("Typing password...");
                        if let Err(e) = pass_input.type_into(password) {
                            error!("Failed to type password: {:?}", e);
                            if let Ok(html) = tab.get_content() {
                                error!("Page HTML at failure to type password:\n{}", html);
                            }
                            if config.keep_open {
                                std::thread::sleep(Duration::from_secs(60));
                            }
                            return Err(anyhow::anyhow!("Failed to type password"));
                        }
                        info!("Successfully typed password.");
                    }
                    Err(e) => {
                        error!("Failed to find password input: {:?}", e);
                        if let Ok(html) = tab.get_content() {
                            error!("Page HTML at failure to find password input:\n{}", html);
                        }
                        if config.keep_open {
                            std::thread::sleep(Duration::from_secs(60));
                        }
                        return Err(anyhow::anyhow!("Failed to find password input"));
                    }
                }
            }

            info!("Waiting for login button...");
            let button_selector = "button#submitFocus, button.pmry";
            match tab.wait_for_element(button_selector) {
                Ok(login_button) => {
                    info!("Clicking login button...");
                    if let Err(e) = login_button.click() {
                        error!("Failed to click login button: {:?}", e);
                        if let Ok(html) = tab.get_content() {
                            error!("Page HTML after failed login click:\n{}", html);
                        }

                        if config.keep_open {
                            std::thread::sleep(Duration::from_secs(60));
                        }
                        return Err(anyhow::anyhow!("Failed to click login button"));
                    }
                    info!("Successfully clicked login button.");
                    if let Ok(html) = tab.get_content() {
                        info!("Page HTML after clicking login:\n{}", html);
                    }
                }
                Err(e) => {
                    error!("Failed to find login button: {:?}", e);
                    if let Ok(html) = tab.get_content() {
                        error!("Page HTML at failure to find login button:\n{}", html);
                    }
                    if config.keep_open {
                        std::thread::sleep(Duration::from_secs(60));
                    }
                    return Err(anyhow::anyhow!("Failed to find login button"));
                }
            }

            info!("Waiting for dashboard to load or error message...");
            let mut login_success = false;
            for _ in 0..15 {
                // Wait up to 30 seconds (15 * 2s)
                if let Ok(err_alert) = tab.find_element(".alert-danger.fade.show") {
                    let msg = err_alert.get_inner_text().unwrap_or_default();
                    error!("Login failed: {}", msg.trim());
                    if let Ok(html) = tab.get_content() {
                        error!("Page HTML after failed login:\n{}", html);
                    }

                    if config.keep_open {
                        std::thread::sleep(Duration::from_secs(60));
                    }
                    return Err(anyhow::anyhow!("Login failed: {}", msg.trim()));
                }

                if let Ok(usr_id_element) = tab.find_element("span.usr-id") {
                    login_success = true;
                    let inner_text = usr_id_element.get_inner_text().unwrap_or_default();
                    if inner_text.contains(&config.username) {
                        info!(
                            "Successfully verified username {} on the page.",
                            config.username
                        );
                        println!("Verified username {} on the page.", config.username);
                    } else {
                        error!(
                            "Username mismatch! Found '{}', expected '{}'",
                            inner_text, config.username
                        );
                        if let Ok(html) = tab.get_content() {
                            error!("Page HTML at username verification mismatch:\n{}", html);
                        }
                    }
                    break;
                }
                std::thread::sleep(Duration::from_secs(2));
            }

            if !login_success {
                error!("Failed to reach dashboard or find error message");
                if let Ok(html) = tab.get_content() {
                    error!("Page HTML at dashboard timeout:\n{}", html);
                }

                if config.keep_open {
                    std::thread::sleep(Duration::from_secs(60));
                }
                return Err(anyhow::anyhow!("Dashboard timeout"));
            }

            info!("Waiting for menu to fully render...");
            std::thread::sleep(Duration::from_secs(2)); // Short delay for Angular to stabilize
            let mut menu_found = false;
            for _ in 0..10 {
                // Wait up to 20 seconds (10 * 2s)
                if tab.find_element("#menuPinnedBtn").is_ok() {
                    menu_found = true;
                    break;
                }
                std::thread::sleep(Duration::from_secs(2));
            }
            if !menu_found {
                error!("Menu #menuPinnedBtn not found after wait.");
                if let Ok(html) = tab.get_content() {
                    error!("Page HTML at menu wait timeout:\n{}", html);
                }

                if config.keep_open {
                    std::thread::sleep(Duration::from_secs(60));
                }
                return Err(anyhow::anyhow!("Menu wait timeout"));
            }

            info!("Attempting to open menu and find MIS module...");
            let js_click_menu = r#"
                (function() {
                    try {
                        var clicked = false;
                        var btn = document.querySelector('#menuPinnedBtn');
                        if (btn) {
                            btn.click();
                            clicked = true;
                        }
                        return clicked ? "CLICKED" : "NOT_FOUND";
                    } catch (e) {
                        return "ERROR: " + e.message;
                    }
                })();
            "#;

            let mis_selector = ".menu-grid-item.misManagement";
            let mut mis_found = false;

            for attempt in 1..=10 {
                info!("Menu open attempt {}/10...", attempt);
                let mut menu_clicked = false;

                if let Ok(eval_result) = tab.evaluate(js_click_menu, true) {
                    if let Some(val) = eval_result.value {
                        if let Some(val_str) = val.as_str() {
                            if val_str == "CLICKED" {
                                info!("Successfully clicked #menuPinnedBtn via JS.");
                                menu_clicked = true;
                            } else {
                                error!("Failed to click #menuPinnedBtn via JS: {}", val_str);
                            }
                        }
                    }
                }

                if !menu_clicked {
                    // Fallback to native click
                    match tab.wait_for_element("#menuPinnedBtn") {
                        Ok(menu_btn) => {
                            if let Err(e) = menu_btn.click() {
                                error!("Failed to click #menuPinnedBtn: {:?}", e);
                            } else {
                                info!("Successfully clicked #menuPinnedBtn (fallback native).");
                                menu_clicked = true;
                            }
                        }
                        Err(e) => {
                            error!("Failed to find #menuPinnedBtn for fallback click: {:?}", e);
                        }
                    }
                }

                if menu_clicked {
                    // Wait for the menu to visually open (menuModules gets show-modules class)
                    info!("Waiting for the pinned menu to fully open (show-modules class)...");
                    let mut sidebar_toggled = false;
                    for check_idx in 0..15 {
                        let check_js = r#"
                            (function() {
                                var menuModules = document.querySelector('.menuModules');
                                return menuModules && menuModules.classList.contains('show-modules');
                            })();
                        "#;
                        if let Ok(eval_result) = tab.evaluate(check_js, true) {
                            if let Some(val) = eval_result.value {
                                if let Some(is_toggled) = val.as_bool() {
                                    info!(
                                        "Check {} for show-modules: {}",
                                        check_idx + 1,
                                        is_toggled
                                    );
                                    if is_toggled {
                                        sidebar_toggled = true;
                                        break;
                                    }
                                }
                            }
                        }
                        std::thread::sleep(Duration::from_millis(1000));
                    }

                    if !sidebar_toggled {
                        warn!("Menu '.menuModules' did not receive 'show-modules' class after waiting. MIS Reports might be inaccessible.");
                        let log_classes_js = "document.querySelector('.menuModules') ? document.querySelector('.menuModules').className : 'NOT_FOUND'";
                        if let Ok(eval_result) = tab.evaluate(log_classes_js, true) {
                            if let Some(val) = eval_result.value {
                                if let Some(classes) = val.as_str() {
                                    warn!("Current .menuModules classes: {}", classes);
                                }
                            }
                        }
                    } else {
                        info!("Menu successfully opened.");
                    }

                    // Wait for MIS module to appear in DOM (it usually is there, but just to be sure)
                    info!("Waiting for MIS module to be present in DOM...");
                    for _ in 0..5 {
                        if tab.find_element(mis_selector).is_ok() {
                            mis_found = true;
                            break;
                        }
                        std::thread::sleep(Duration::from_secs(1));
                    }
                }

                if mis_found {
                    break;
                } else {
                    info!("MIS module not found in attempt {}, retrying...", attempt);
                }
            }

            // Re-creating the original match to not break the brace structure down below
            match Ok::<(), ()>(()) {
                Ok(_) => {
                    if !mis_found {
                        error!("MIS module not found after all attempts.");
                        if let Ok(html) = tab.get_content() {
                            error!("Page HTML at MIS module wait timeout:\n{}", html);
                        }
                        if config.keep_open {
                            std::thread::sleep(Duration::from_secs(60));
                        }
                        return Err(anyhow::anyhow!("MIS module wait timeout"));
                    } else {
                        match tab.wait_for_element(mis_selector) {
                            Ok(mis_module) => {
                                info!("Clicking on MIS module...");
                                if let Err(e) = mis_module.click() {
                                    error!("Failed to click MIS module: {:?}", e);
                                    if let Ok(html) = tab.get_content() {
                                        error!(
                                            "Page HTML after failed MIS module click:\n{}",
                                            html
                                        );
                                    }
                                } else {
                                    info!("Clicked MIS successfully. Waiting for MIS Reports button...");

                                    let mut mis_reports_found = false;
                                    let mis_reports_xpath = "//div[contains(@class, 'label') and contains(@class, 'fw-bold') and contains(text(), 'MIS Reports')]";

                                    for _ in 0..10 {
                                        // Wait up to 20 seconds (10 * 2s)
                                        if tab.find_element_by_xpath(mis_reports_xpath).is_ok() {
                                            mis_reports_found = true;
                                            break;
                                        }
                                        std::thread::sleep(Duration::from_secs(2));
                                    }

                                    if !mis_reports_found {
                                        error!("MIS Reports button not found after wait.");
                                        if let Ok(html) = tab.get_content() {
                                            error!(
                                                "Page HTML at MIS Reports button wait timeout:\n{}",
                                                html
                                            );
                                        }
                                        if config.keep_open {
                                            std::thread::sleep(Duration::from_secs(60));
                                        }
                                        return Err(anyhow::anyhow!(
                                            "MIS Reports button wait timeout"
                                        ));
                                    }

                                    info!("MIS Reports button successfully verified. MIS module click was successful.");
                                    println!("MIS Reports button successfully verified. MIS module click was successful.");
                                    if let Ok(html) = tab.get_content() {
                                        info!(
                                            "Page HTML after MIS Reports verification:\n{}",
                                            html
                                        );
                                    }

                                    // Let the page settle
                                    std::thread::sleep(Duration::from_secs(2));

                                    // Find iframe
                                    info!("Searching for auto-login iframe...");
                                    let mut iframe_node_id = None;

                                    // Give it a moment to load
                                    for _ in 0..5 {
                                        if let Ok(iframe_node) = tab.find_element("iframe") {
                                            iframe_node_id = Some(iframe_node.node_id);
                                            break;
                                        }
                                        std::thread::sleep(Duration::from_secs(2));
                                    }

                                    if iframe_node_id.is_none() {
                                        error!("Could not find iframe.");
                                        return Err(anyhow::anyhow!("Iframe not found"));
                                    }

                                    info!("Running full JS automation sequence...");

                                    let filters_json = serde_json::to_string(active_filters)
                                        .unwrap_or_else(|_| "{}".to_string());

                                    // We will use evaluate but because of cross origin, we need the
                                    // `--disable-web-security` flag to work, or we try to run it inside the specific frame.
                                    // Since we added `--disable-web-security`, accessing `iframe.contentWindow.document` should work!

                                    let js_script = format!(
                                        r#"
                                        (async function(reportType, reportName, filters) {{
                                            function sleep(ms) {{ return new Promise(r => setTimeout(r, ms)); }}

                                            async function simulateTyping(inputElem, text) {{
                                                inputElem.focus();
                                                inputElem.value = ''; // clear first

                                                for (let i = 0; i < text.length; i++) {{
                                                    inputElem.value += text[i];
                                                    inputElem.dispatchEvent(new Event('input', {{ bubbles: true }}));
                                                    await sleep(10);
                                                }}

                                                inputElem.dispatchEvent(new Event('change', {{ bubbles: true }}));
                                                inputElem.dispatchEvent(new KeyboardEvent('keydown', {{ key: 'Enter', code: 'Enter', keyCode: 13, which: 13, bubbles: true }}));
                                                inputElem.dispatchEvent(new KeyboardEvent('keyup', {{ key: 'Enter', code: 'Enter', keyCode: 13, which: 13, bubbles: true }}));
                                                inputElem.blur();
                                                inputElem.dispatchEvent(new Event('blur', {{ bubbles: true }}));
                                            }}

                                            let iframe = document.querySelector('iframe');
                                            if (!iframe) return "ERROR: No iframe found.";
                                            let doc;
                                            try {{
                                                doc = iframe.contentWindow.document;
                                            }} catch (e) {{
                                                return "ERROR: Cross origin blocked.";
                                            }}

                                            // 1. Select Report Type
                                            let xpathType = `//*[contains(text(), '${{reportType}}')]/ancestor-or-self::mat-radio-button`;
                                            let resultType = doc.evaluate(xpathType, doc, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                                            let matRadioButton = resultType.singleNodeValue;

                                            let clickedType = false;
                                            if (matRadioButton) {{
                                                let innerInput = matRadioButton.querySelector('input[type="radio"]');
                                                if (innerInput) {{ innerInput.click(); clickedType = true; }}
                                                else {{ matRadioButton.click(); clickedType = true; }}
                                            }} else {{
                                                let fallbackXpath = `//label[contains(text(), '${{reportType}}')]`;
                                                let fallbackResult = doc.evaluate(fallbackXpath, doc, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                                                let labelNode = fallbackResult.singleNodeValue;
                                                if (labelNode) {{ labelNode.click(); clickedType = true; }}
                                            }}

                                            if (!clickedType) return "ERROR: Report type not found: " + reportType;

                                            // Wait for list
                                            let listLoaded = false;
                                            for (let i = 0; i < 20; i++) {{
                                                let divs = doc.querySelectorAll('div.fw-semibold');
                                                for (let d of divs) {{
                                                    let textLower = d.textContent.toLowerCase();
                                                    if (textLower.includes('report manager') || textLower.includes('report manger') || textLower.includes(reportType.toLowerCase()) || textLower.includes('enquiry') || textLower.includes('standard report')) {{
                                                        listLoaded = true; break;
                                                    }}
                                                }}
                                                if (listLoaded) break;
                                                await sleep(500);
                                            }}

                                            if (!listLoaded) return "ERROR: Report list timeout.";

                                            // 2. Search Report Name
                                            await sleep(1000);
                                            let searchInputList = doc.querySelector('input[formcontrolname="searchInput"], input[placeholder="Search"]');
                                            if (searchInputList) {{
                                                searchInputList.value = reportName;
                                                searchInputList.dispatchEvent(new Event('input', {{ bubbles: true }}));
                                                searchInputList.dispatchEvent(new Event('change', {{ bubbles: true }}));
                                                searchInputList.dispatchEvent(new KeyboardEvent('keyup', {{ bubbles: true }}));
                                            }}

                                            let reportFound = false;
                                            for (let i = 0; i < 20; i++) {{
                                                let itemXpath = `//li[contains(@class, 'sub-list-items')]//span[contains(text(), '${{reportName}}')]`;
                                                let itemResult = doc.evaluate(itemXpath, doc, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                                                let reportSpan = itemResult.singleNodeValue;
                                                if (reportSpan) {{
                                                    let liElement = reportSpan.closest('li.sub-list-items');
                                                    if (liElement) liElement.click(); else reportSpan.click();
                                                    reportFound = true;
                                                    break;
                                                }}
                                                await sleep(1500);
                                            }}

                                            if (!reportFound) return "ERROR: Report name not found: " + reportName;

                                            // Wait binding
                                            let reportBound = false;
                                            for (let i = 0; i < 30; i++) {{
                                                let selects = doc.querySelectorAll('mat-select');
                                                for (let s of selects) {{
                                                    if (s.innerText.includes(reportName) || s.textContent.includes(reportName)) {{
                                                        reportBound = true; break;
                                                    }}
                                                }}
                                                if (reportBound) break;
                                                await sleep(1500);
                                            }}

                                            // 3. Extract Dynamic Filters
                                            let labels = doc.querySelectorAll('mat-label');
                                            let discoveredFilters = [];
                                            for (let lbl of labels) {{
                                                if (lbl.innerText) {{
                                                    discoveredFilters.push(lbl.innerText.trim());
                                                }}
                                            }}

                                            // 4. Fill Dynamic Filters
                                            for (const [key, value] of Object.entries(filters)) {{
                                                for (let lbl of labels) {{
                                                    if (lbl.innerText.trim().toLowerCase() === key.toLowerCase()) {{
                                                        let labelParent = lbl.closest('label');
                                                        if (labelParent && labelParent.hasAttribute('for')) {{
                                                            let inputId = labelParent.getAttribute('for');
                                                            let input = doc.getElementById(inputId);
                                                            if (input && input.tagName === 'INPUT') {{
                                                                let v = value;
                                                                if (key.toLowerCase().includes('date') && v.includes('-')) {{
                                                                    let parts = v.split(' ')[0].split('-');
                                                                    if (parts.length === 3) {{
                                                                        let d = parts[0].padStart(2, '0');
                                                                        let m = parts[1].padStart(2, '0');
                                                                        let y = parts[2];
                                                                        v = d + "-" + m + "-" + y + (v.includes(' ') ? ' ' + v.split(' ').slice(1).join(' ') : '');
                                                                    }}
                                                                }}
                                                                await simulateTyping(input, v);
                                                                break;
                                                            }}
                                                        }}
                                                    }}
                                                }}
                                            }}

                                            await sleep(1000);
                                            let searchBtnIcon = doc.querySelector('button.btn-primary i.bi-search');
                                            if (searchBtnIcon) searchBtnIcon.closest('button').click();

                                            let loaderAppeared = false;
                                            for(let i=0; i<10; i++) {{
                                                let loader = doc.querySelector('.loading-screen-wrapper, mat-progress-bar');
                                                if (loader && loader.offsetParent !== null) {{ loaderAppeared = true; break; }}
                                                await sleep(1000);
                                            }}

                                            if (loaderAppeared) {{
                                                for(let i=0; i<120; i++) {{
                                                    let loader = doc.querySelector('.loading-screen-wrapper, mat-progress-bar');
                                                    if (!loader || loader.offsetParent === null) break;
                                                    await sleep(1500);
                                                }}
                                            }}

                                            await sleep(2000);
                                            let exportBtn = null;
                                            let dxButtons = doc.querySelectorAll('.dx-button-text');
                                            for (let btn of dxButtons) {{
                                                if (btn.textContent.trim() === 'Export') {{ exportBtn = btn.closest('div[role=\"button\"]'); break; }}
                                            }}

                                            if (exportBtn) {{
                                                exportBtn.click();
                                                await sleep(1000);
                                                let xlsxOption = null;
                                                let listItems = doc.querySelectorAll('.dx-list-item-content');
                                                for (let item of listItems) {{
                                                    if (item.textContent.trim() === 'XLSX') {{ xlsxOption = item.closest('.dx-list-item'); break; }}
                                                }}
                                                if (xlsxOption) xlsxOption.click();
                                            }}

                                            let finalResult = {{
                                                status: "SUCCESS: Automation Complete",
                                                discovered_filters: discoveredFilters
                                            }};
                                            return JSON.stringify(finalResult);
                                        }})('{}', '{}', {});
                                        "#,
                                        active_report_type, active_report_name, filters_json
                                    );

                                    info!("Evaluating JS to execute automation sequence...");
                                    match tab.evaluate(&js_script, true) {
                                        Ok(res) => {
                                            if let Some(v) = res.value {
                                                let v_str = v.as_str().unwrap_or("");
                                                info!("JS Result: {}", v_str);
                                                if v_str == "ERROR: Cross origin blocked." {
                                                    error!("Cross origin blocked. --disable-web-security failed.");
                                                } else if v_str.starts_with("ERROR") {
                                                    error!("JS Automation Error: {}", v_str);
                                                } else {
                                                    // Parse the JSON result
                                                    if let Ok(parsed_res) =
                                                        serde_json::from_str::<serde_json::Value>(
                                                            v_str,
                                                        )
                                                    {
                                                        if let Some(filters_arr) = parsed_res
                                                            .get("discovered_filters")
                                                            .and_then(|f| f.as_array())
                                                        {
                                                            for filter in filters_arr {
                                                                if let Some(f_str) = filter.as_str()
                                                                {
                                                                    discovered_filters
                                                                        .push(f_str.to_string());
                                                                }
                                                            }
                                                            info!(
                                                                "Discovered filters natively: {:?}",
                                                                discovered_filters
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => error!("Evaluate typing failed: {}", e),
                                    }

                                    std::thread::sleep(Duration::from_secs(5));
                                }
                            }
                            Err(e) => {
                                error!("Failed to find MIS module: {:?}", e);
                                if let Ok(html) = tab.get_content() {
                                    error!("Page HTML at failure to find MIS module:\n{}", html);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to find #menuPinnedBtn: {:?}", e);
                    if let Ok(html) = tab.get_content() {
                        error!("Page HTML at failure to find #menuPinnedBtn:\n{}", html);
                    }
                }
            }
        }
        Err(e) => {
            error!(
                "Failed to find username input, likely because page did not load: {:?}",
                e
            );
            if let Ok(html) = tab.get_content() {
                error!("Page HTML at failure to find username:\n{}", html);
            }

            if config.keep_open {
                std::thread::sleep(Duration::from_secs(60));
            }
            return Err(anyhow::anyhow!("Failed to find elements to login"));
        }
    }

    // Wait for download if applicable
    if let Some(dl_dir) = download_dir {
        info!("Waiting for download to complete in {:?}...", dl_dir);
        let mut download_complete = false;
        // Wait up to 3 minutes
        for _ in 0..180 {
            if let Ok(entries) = std::fs::read_dir(&dl_dir) {
                let mut found_incomplete = false;
                let mut found_completed = false;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "crdownload" || ext == "tmp" {
                            found_incomplete = true;
                        } else if ext == "xlsx" || ext == "xls" || ext == "csv" {
                            found_completed = true;
                        }
                    }
                }

                if found_completed && !found_incomplete {
                    download_complete = true;
                    break;
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }

        if download_complete {
            info!("Download successfully completed in {:?}", dl_dir);
        } else {
            error!("Download wait timeout or failed in {:?}", dl_dir);
        }
    }

    // Remove listener before exit
    tab.remove_event_listener(&events)
        .context("Failed to remove listener")?;

    if config.keep_open {
        std::thread::sleep(Duration::from_secs(60));
    }

    // Cleanup tab if it is not the only tab left
    if !is_initial_tab {
        if let Err(e) = tab.close(true) {
            error!("Failed to close tab: {:?}", e);
        }
    }

    Ok(discovered_filters)
}

fn print_manifest(config_path: Option<PathBuf>) {
    let mut report_names = Vec::new();
    let mut filter_dependencies: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut type_autofills: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut filter_autofills: std::collections::HashMap<
        String,
        std::collections::HashMap<String, String>,
    > = std::collections::HashMap::new();

    if let Some(path) = config_path.clone() {
        if let Ok(config_str) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<YaswebConfig>(&config_str) {
                for (name, report) in &config.reports {
                    report_names.push(name.clone());

                    if !report.report_type.trim().is_empty() {
                        type_autofills.insert(name.clone(), report.report_type.clone());
                    }

                    let start_key = report.start_date_key.clone().unwrap_or_default();
                    let end_key = report.end_date_key.clone().unwrap_or_default();

                    for (filter_key, filter_val) in &report.filters {
                        // Skip generating filter args for standard date keys since we map --start-date/--end-date to them
                        if filter_key == &start_key || filter_key == &end_key {
                            continue;
                        }
                        filter_dependencies
                            .entry(filter_key.clone())
                            .or_default()
                            .push(name.clone());

                        if !filter_val.trim().is_empty() {
                            filter_autofills
                                .entry(filter_key.clone())
                                .or_default()
                                .insert(name.clone(), filter_val.clone());
                        }
                    }
                }
            }
        }
    }

    let mut arguments = vec![
        AppArg {
            name: "--config".to_string(),
            arg_type: ArgType::String,
            required: false,
            default_value: None,
            options: None,
            depends_on: None,
            autofill: None,
        },
        AppArg {
            name: "--name".to_string(),
            arg_type: if report_names.is_empty() {
                ArgType::String
            } else {
                ArgType::List
            },
            required: true,
            default_value: None,
            options: if report_names.is_empty() {
                None
            } else {
                Some(report_names)
            },
            depends_on: None,
            autofill: None,
        },
        AppArg {
            name: "--type".to_string(),
            arg_type: ArgType::List,
            required: false,
            default_value: None,
            options: Some(vec![
                "".to_string(),
                "standard report".to_string(),
                "report manager".to_string(),
            ]),
            depends_on: None,
            autofill: if type_autofills.is_empty() {
                None
            } else {
                let mut map = std::collections::HashMap::new();
                map.insert("--name".to_string(), type_autofills);
                Some(map)
            },
        },
        AppArg {
            name: "--monthly".to_string(),
            arg_type: ArgType::Boolean,
            required: false,
            default_value: None,
            options: None,
            depends_on: None,
            autofill: None,
        },
        AppArg {
            name: "--start-date".to_string(),
            arg_type: ArgType::String,
            required: false,
            default_value: None,
            options: None,
            depends_on: None,
            autofill: None,
        },
        AppArg {
            name: "--end-date".to_string(),
            arg_type: ArgType::String,
            required: false,
            default_value: None,
            options: None,
            depends_on: None,
            autofill: None,
        },
    ];

    let mut sorted_filters: Vec<_> = filter_dependencies.into_iter().collect();
    sorted_filters.sort_by(|a, b| a.0.cmp(&b.0));
    for (f_name, deps) in sorted_filters {
        let mut depends_map = std::collections::HashMap::new();
        depends_map.insert("--name".to_string(), deps);

        let autofill_opt = if let Some(fills) = filter_autofills.get(&f_name) {
            let mut map = std::collections::HashMap::new();
            map.insert("--name".to_string(), fills.clone());
            Some(map)
        } else {
            None
        };

        arguments.push(AppArg {
            name: format!("--filter-{}", f_name),
            arg_type: ArgType::String,
            required: false,
            default_value: None,
            options: None,
            depends_on: Some(depends_map),
            autofill: autofill_opt,
        });
    }

    let manifest = AppManifest {
        name: "Yasweb Automation Engine".to_string(),
        description: "Executes headless browser automation for web reporting.".to_string(),
        arguments,
    };
    if let Ok(json) = serde_json::to_string(&manifest) {
        println!("{}", json);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Intercept --manifest before anything else
    if args.iter().any(|arg| arg == "--manifest") {
        let mut config_path = None;
        let mut j = 1;
        while j < args.len() {
            if args[j] == "--config" && j + 1 < args.len() {
                let p = PathBuf::from(&args[j + 1]);
                let p = if p.is_absolute() {
                    p
                } else {
                    let mut exe_dir = std::env::current_exe().unwrap_or_default();
                    exe_dir.pop();
                    exe_dir.join(p)
                };
                config_path = Some(p);
                break;
            }
            j += 1;
        }

        if config_path.is_none() {
            let mut default_path = std::env::current_exe().unwrap_or_default();
            default_path.pop();
            default_path.push("yasweb_config.json");
            config_path = Some(default_path);
        }

        print_manifest(config_path);
        std::process::exit(0);
    }

    let _guard = setup_logging()?;

    let args: Vec<String> = std::env::args().collect();

    let mut config_path = None;
    let mut j = 1;
    while j < args.len() {
        if args[j] == "--config" && j + 1 < args.len() {
            config_path = Some(PathBuf::from(&args[j + 1]));
            break;
        }
        j += 1;
    }

    let config_path = match config_path {
        Some(p) => {
            if p.is_absolute() {
                p
            } else {
                let mut exe_dir =
                    std::env::current_exe().context("Failed to get executable path")?;
                exe_dir.pop();
                exe_dir.join(p)
            }
        }
        None => {
            let mut default_path =
                std::env::current_exe().context("Failed to get executable path")?;
            default_path.pop();
            default_path.push("yasweb_config.json");
            default_path
        }
    };

    let mut config = load_or_create_config(&config_path).await?;
    let mut config_updated = false;

    let mut active_report_name = String::new();
    let mut active_report_type = String::new();
    let mut active_filters = HashMap::new();
    let mut is_monthly = false;
    let mut start_date_str = None;
    let mut end_date_str = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--config" if i + 1 < args.len() => {
                i += 1;
            }
            "--type" if i + 1 < args.len() => {
                active_report_type = args[i + 1].clone();
                i += 1;
            }
            "--name" if i + 1 < args.len() => {
                active_report_name = args[i + 1].clone();
                i += 1;
            }
            "--filters" if i + 1 < args.len() => {
                let filters_str = args[i + 1].clone();
                if filters_str.trim().is_empty() {
                    active_filters = HashMap::new();
                } else {
                    match serde_json::from_str::<HashMap<String, String>>(&filters_str) {
                        Ok(parsed_filters) => {
                            active_filters = parsed_filters;
                        }
                        Err(e) => {
                            error!("Failed to parse filters JSON: {}", e);
                            anyhow::bail!("Failed to parse filters JSON: {}", e);
                        }
                    }
                }
                i += 1;
            }
            "--monthly" if i + 1 < args.len() => {
                if args[i + 1].to_lowercase() == "true" || args[i + 1] == "1" {
                    is_monthly = true;
                }
                i += 1;
            }
            "--start-date" if i + 1 < args.len() => {
                start_date_str = Some(args[i + 1].clone());
                i += 1;
            }
            "--end-date" if i + 1 < args.len() => {
                end_date_str = Some(args[i + 1].clone());
                i += 1;
            }
            "--type" | "--name" | "--filters" | "--config" | "--monthly" | "--start-date"
            | "--end-date" => {}
            _ => {}
        }
        i += 1;
    }

    if active_report_name.is_empty() {
        error!("Validation failed: --name is required.");
        anyhow::bail!("Validation failed: --name is required.");
    }

    // Determine configuration to use
    if !active_report_type.is_empty() || !active_filters.is_empty() {
        // We received details from CLI.
        let report_conf = ReportConfig {
            report_type: active_report_type.clone(),
            filters: active_filters.clone(),
            start_date_key: None,
            end_date_key: None,
        };
        config
            .reports
            .insert(active_report_name.clone(), report_conf);
        config_updated = true;
    } else {
        // Retrieve from config
        if let Some(cached) = config.reports.get(&active_report_name) {
            info!("Found cached configuration for '{}'", active_report_name);
            active_report_type = cached.report_type.clone();
            active_filters = cached.filters.clone();
        } else {
            error!(
                "Report '{}' not found in config and no --type/--filters provided via CLI.",
                active_report_name
            );
            anyhow::bail!(
                "Report '{}' not found in config and no --type/--filters provided via CLI.",
                active_report_name
            );
        }
    }

    if config_updated {
        info!("Updating yasweb_config.json with CLI report parameters...");
        let content = serde_json::to_string_pretty(&config)
            .context("Failed to serialize updated yasweb config")?;
        fs::write(&config_path, content)
            .await
            .context("Failed to write updated yasweb_config.json")?;
    }

    info!("Loaded config for URL: {}", config.url);

    // Parse date ranges
    let mut date_ranges: Vec<(String, String)> = Vec::new();
    if is_monthly {
        let start_date = start_date_str
            .clone()
            .context("--start-date is required when --monthly is true")?;
        let end_date = end_date_str
            .clone()
            .context("--end-date is required when --monthly is true")?;

        let start_dt = NaiveDate::parse_from_str(&start_date, "%d-%m-%Y")
            .context("Invalid --start-date format, expected DD-MM-YYYY")?;
        let end_dt = NaiveDate::parse_from_str(&end_date, "%d-%m-%Y")
            .context("Invalid --end-date format, expected DD-MM-YYYY")?;

        if start_dt > end_dt {
            anyhow::bail!("--start-date must be before or equal to --end-date");
        }

        let report_conf = config
            .reports
            .get(&active_report_name)
            .context("Report name not found in config")?;
        if report_conf.start_date_key.is_none()
            || report_conf.end_date_key.is_none()
            || report_conf
                .start_date_key
                .as_ref()
                .is_none_or(|k| k.is_empty())
            || report_conf
                .end_date_key
                .as_ref()
                .is_none_or(|k| k.is_empty())
        {
            error!("For monthly execution, the report must have start_date_key and end_date_key configured in yasweb_config.json.");
            anyhow::bail!("For monthly execution, the report must have start_date_key and end_date_key configured in yasweb_config.json.");
        }

        let mut current_dt = start_dt;
        while current_dt <= end_dt {
            let next_month = if current_dt.month() == 12 {
                NaiveDate::from_ymd_opt(current_dt.year() + 1, 1, 1).context("Invalid date math")?
            } else {
                NaiveDate::from_ymd_opt(current_dt.year(), current_dt.month() + 1, 1)
                    .context("Invalid date math")?
            };

            let last_day = next_month.pred_opt().context("Invalid date math")?;
            let chunk_end = if last_day > end_dt { end_dt } else { last_day };

            date_ranges.push((
                current_dt.format("%d-%m-%Y").to_string(),
                chunk_end.format("%d-%m-%Y").to_string(),
            ));

            current_dt = next_month;
        }

        info!(
            "Monthly execution planned for {} ranges: {:?}",
            date_ranges.len(),
            date_ranges
        );
    } else {
        // Not monthly, just add a dummy single entry
        date_ranges.push(("".to_string(), "".to_string()));
    }

    let config_path_clone = config_path.clone();
    let mut config_clone = config.clone();
    let active_report_name_clone = active_report_name.clone();

    // Launch browser once
    let mut user_data_dir = std::env::current_exe().unwrap_or_default();
    user_data_dir.pop();
    user_data_dir.push("yasweb_chrome_data");

    let args = vec![
        std::ffi::OsStr::new("--ignore-certificate-errors"),
        std::ffi::OsStr::new("--start-maximized"),
        std::ffi::OsStr::new("--disable-web-security"),
        std::ffi::OsStr::new("--disable-site-isolation-trials"),
        std::ffi::OsStr::new("--disable-features=IsolateOrigins,site-per-process"),
    ];

    let launch_options = LaunchOptions::default_builder()
        .headless(config.headless)
        .sandbox(false)
        .idle_browser_timeout(std::time::Duration::from_secs(120))
        .user_data_dir(Some(user_data_dir))
        .args(args)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build launch options: {e}"))?;

    let browser = Arc::new(Browser::new(launch_options).context("Failed to launch browser")?);

    // Chunk runs into concurrent batches of max 6
    for chunk in date_ranges.chunks(6) {
        let mut tasks = Vec::new();
        for (i, (start_dt, end_dt)) in chunk.iter().enumerate() {
            let config_task = config.clone();
            let active_report_name_task = active_report_name.clone();
            let active_report_type_task = active_report_type.clone();

            let mut run_filters = active_filters.clone();
            let report_conf_opt = config.reports.get(&active_report_name);

            // Map the global --start-date and --end-date values into the respective internal report filters
            // This is done both for monthly chunking AND standard executions, provided the report has the keys configured.
            let s_dt = if is_monthly && !start_dt.is_empty() {
                Some(start_dt.clone())
            } else {
                start_date_str.clone()
            };
            let e_dt = if is_monthly && !end_dt.is_empty() {
                Some(end_dt.clone())
            } else {
                end_date_str.clone()
            };

            if let Some(report_conf) = report_conf_opt {
                if let Some(sk) = &report_conf.start_date_key {
                    if let Some(s) = s_dt.clone() {
                        run_filters.insert(sk.clone(), s);
                    }
                }
                if let Some(ek) = &report_conf.end_date_key {
                    if let Some(e) = e_dt.clone() {
                        run_filters.insert(ek.clone(), e);
                    }
                }
            }

            let browser_clone = browser.clone();
            let is_initial = i == 0 && !chunk.is_empty(); // use initial tab for the first element in the batch to avoid lingering blank tabs

            // Setup download dir for this tab
            let date_suffix = if is_monthly && !start_dt.is_empty() {
                // e.g. "01-01-2026", get MM-YYYY
                let parts: Vec<&str> = start_dt.split('-').collect();
                if parts.len() == 3 {
                    format!("{}-{}", parts[1], parts[2])
                } else {
                    start_dt.clone()
                }
            } else if let Some(st) = start_date_str.clone() {
                st
            } else {
                chrono::Local::now().format("%d-%m-%Y").to_string()
            };

            let temp_dl_dir = {
                let mut dir = std::env::current_exe().unwrap_or_default();
                dir.pop();
                dir.push(format!(
                    "yasweb_downloads_tmp_{}_{}_{}",
                    active_report_name, date_suffix, i
                ));
                let _ = std::fs::create_dir_all(&dir);
                dir
            };

            let temp_dl_dir_clone = temp_dl_dir.clone();
            let final_filename = format!("{}_{}.xlsx", active_report_name, date_suffix);

            tasks.push(tokio::task::spawn_blocking(move || {
                let res = match run_browser_tab(
                    browser_clone,
                    &config_task,
                    &active_report_name_task,
                    &active_report_type_task,
                    &run_filters,
                    is_initial,
                    Some(temp_dl_dir.clone()),
                ) {
                    Ok(filters) => filters,
                    Err(e) => {
                        error!("Browser automation failed: {:?}", e);
                        eprintln!("Browser automation failed: {:?}", e);
                        Vec::new()
                    }
                };
                (res, temp_dl_dir_clone, final_filename)
            }));
        }

        let results = futures_util::future::join_all(tasks).await;

        for (discovered_filters, temp_dl_dir, final_filename) in results.into_iter().flatten() {
            // Move and rename the file
            if let Ok(entries) = std::fs::read_dir(&temp_dl_dir) {
                let mut final_out_dir = std::env::current_exe().unwrap_or_default();
                final_out_dir.pop();
                final_out_dir.push("downloads");
                let _ = std::fs::create_dir_all(&final_out_dir);

                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "xlsx" || ext == "xls" || ext == "csv" {
                            let mut out_file = final_out_dir.clone();
                            // If the original file was not xlsx, adapt the extension, but we requested XLSX
                            let ext_str = ext.to_string_lossy().to_string();
                            let mut final_name = final_filename.clone();
                            if ext_str != "xlsx" {
                                final_name = final_name.replace(".xlsx", &format!(".{}", ext_str));
                            }
                            out_file.push(final_name);

                            info!("Moving downloaded file from {:?} to {:?}", path, out_file);
                            if let Err(e) = std::fs::rename(&path, &out_file) {
                                error!("Failed to rename/move file: {}", e);
                                // Fallback to copy+delete across mount points
                                if std::fs::copy(&path, &out_file).is_ok() {
                                    let _ = std::fs::remove_file(&path);
                                }
                            }
                        }
                    }
                }
            }

            // Cleanup temp dir
            let _ = std::fs::remove_dir_all(&temp_dl_dir);

            if !discovered_filters.is_empty() {
                if let Some(report) = config_clone.reports.get_mut(&active_report_name_clone) {
                    let mut updated_filters = false;
                    for f in discovered_filters {
                        if let std::collections::hash_map::Entry::Vacant(e) =
                            report.filters.entry(f)
                        {
                            e.insert("".to_string());
                            updated_filters = true;
                        }
                    }
                    if updated_filters {
                        info!("Updating yasweb_config.json with newly discovered filters...");
                        let content = serde_json::to_string_pretty(&config_clone)
                            .context("Failed to serialize updated yasweb config")?;
                        // Best effort write
                        let _ = fs::write(&config_path_clone, content).await;
                    }
                }
            }
        }
    }

    Ok(())
}
