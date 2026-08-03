use anyhow::{Context, Result};
use headless_chrome::{protocol::cdp::types::Event, Browser};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info, warn};

use crate::yasweb::config::YaswebConfig;

lazy_static::lazy_static! {
    static ref GLOBAL_DOWNLOAD_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);
}

pub fn get_global_download_dir() -> Arc<Mutex<Option<PathBuf>>> {
    Arc::new(Mutex::new(None)) // To decouple, pass this from caller
}

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

pub fn run_browser_tab(
    browser: Arc<Browser>,
    config: &YaswebConfig,
    active_report_name: &str,
    active_report_type: &str,
    active_filters: &HashMap<String, String>,
    download_dir: Option<PathBuf>,
) -> Result<Vec<String>> {
    let mut discovered_filters = Vec::new();
    let mut step_num = 1;

    let mut found = None;
    for _ in 0..5 {
        let tabs = browser.get_tabs().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(first) = tabs.first() {
            found = Some(first.clone());
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    let tab = match found {
        Some(t) => t,
        None => browser.new_tab().context("Failed to open new tab")?,
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

    info!("Navigating to {}", config.url);
    if let Err(e) = tab.navigate_to(&config.url) {
        error!("Navigate failed: {:?}", e);
        println!(
            "Warning: navigate to {} returned error, continuing anyway...",
            config.url
        );
    } else {
        info!("Successfully navigated to {}", config.url);
        crate::yasweb::browser::save_html_state(
            &tab,
            active_report_name,
            step_num,
            "Main page load",
        );
        step_num += 1;
    }

    // Attempt to wait until navigated, ignore error if it timeouts but page loads
    let _ = tab.wait_until_navigated();

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
                    crate::yasweb::browser::save_html_state(
                        &tab,
                        active_report_name,
                        step_num,
                        "After login",
                    );
                    step_num += 1;
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
                    crate::yasweb::browser::save_html_state(
                        &tab,
                        active_report_name,
                        step_num,
                        "After clicking #menuPinnedBtn",
                    );
                    step_num += 1;

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
                                    if let Ok(html) = tab.get_content() {
                                        tracing::trace!(
                                            "Page HTML immediately after clicking MIS module:\n{}",
                                            html
                                        );
                                    }

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
                                    crate::yasweb::browser::save_html_state(
                                        &tab,
                                        active_report_name,
                                        step_num,
                                        "After clicking MIS module",
                                    );
                                    step_num += 1;
                                    if let Ok(html) = tab.get_content() {
                                        tracing::trace!(
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

                                    let timeout_loops = (config.timeout_minutes * 60) / 10;

                                    // STEP 1: Select Report Type
                                    let step1_js = format!(
                                        r#"
                                        (async function(reportType) {{
                                            function sleep(ms) {{ return new Promise(r => setTimeout(r, ms)); }}
                                            let logs = [];
                                            let iframe = document.querySelector('iframe');
                                            if (!iframe) return JSON.stringify({{ status: "ERROR", msg: "No iframe found." }});
                                            let doc;
                                            try {{
                                                doc = iframe.contentWindow.document;
                                            }} catch (e) {{
                                                return JSON.stringify({{ status: "ERROR", msg: "Cross origin blocked." }});
                                            }}

                                            let clickedType = false;
                                            logs.push("Searching for reportType: " + reportType);

                                            for (let i = 0; i < 20; i++) {{
                                                let xpathType = `//*[contains(text(), '${{reportType}}')]/ancestor-or-self::mat-radio-button`;
                                                let resultType = doc.evaluate(xpathType, doc, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                                                let matRadioButton = resultType.singleNodeValue;

                                                if (matRadioButton) {{
                                                    logs.push("Found mat-radio-button");
                                                    let innerInput = matRadioButton.querySelector('input[type="radio"]');
                                                    if (innerInput) {{
                                                        innerInput.click();
                                                        innerInput.dispatchEvent(new Event('change', {{ bubbles: true }}));
                                                        clickedType = true;
                                                        logs.push("Clicked innerInput");
                                                    }} else {{
                                                        matRadioButton.click();
                                                        clickedType = true;
                                                        logs.push("Clicked matRadioButton");
                                                    }}
                                                }} else {{
                                                    let fallbackXpath = `//label[contains(text(), '${{reportType}}')]`;
                                                    let fallbackResult = doc.evaluate(fallbackXpath, doc, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                                                    let labelNode = fallbackResult.singleNodeValue;
                                                    if (labelNode) {{
                                                        labelNode.click();
                                                        clickedType = true;
                                                        logs.push("Clicked label fallback");
                                                    }}
                                                }}

                                                if (clickedType) break;
                                                await sleep(500);
                                            }}

                                            if (!clickedType) return JSON.stringify({{ status: "ERROR", msg: "Report type not found: " + reportType, logs }});
                                            return JSON.stringify({{ status: "SUCCESS", logs }});
                                        }})({});
                                        "#,
                                        serde_json::to_string(&active_report_type).unwrap()
                                    );

                                    crate::yasweb::browser::save_html_state(
                                        &tab,
                                        active_report_name,
                                        step_num,
                                        "Before selecting Report Type",
                                    );
                                    step_num += 1;
                                    info!("Selecting Report Type: {}", active_report_type);
                                    if let Ok(res) = tab.evaluate(&step1_js, true) {
                                        if let Some(v) = res.value {
                                            if let Some(s) = v.as_str() {
                                                let parsed: serde_json::Value =
                                                    serde_json::from_str(s).unwrap_or_default();
                                                let status = parsed
                                                    .get("status")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("UNKNOWN");
                                                let logs = parsed
                                                    .get("logs")
                                                    .map(|v| v.to_string())
                                                    .unwrap_or_default();
                                                if status == "ERROR" {
                                                    let msg = parsed
                                                        .get("msg")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("Unknown error");
                                                    error!(
                                                        "Step 1 Failed: {} | Logs: {}",
                                                        msg, logs
                                                    );
                                                    return Err(anyhow::anyhow!(
                                                        "Automation Step 1 failed"
                                                    ));
                                                } else {
                                                    info!("Step 1 Success | Logs: {}", logs);
                                                }
                                            }
                                        }
                                    } else {
                                        error!("Failed to evaluate Step 1 JS.");
                                        return Err(anyhow::anyhow!("Automation Step 1 failed"));
                                    }
                                    crate::yasweb::browser::save_html_state(
                                        &tab,
                                        active_report_name,
                                        step_num,
                                        "After Select Report Type",
                                    );
                                    step_num += 1;

                                    // STEP 2: Wait for list & search report
                                    let step2_js = format!(
                                        r#"
                                        (async function(reportType, reportName) {{
                                            function sleep(ms) {{ return new Promise(r => setTimeout(r, ms)); }}
                                            let logs = [];
                                            let iframe = document.querySelector('iframe');
                                            if (!iframe) return JSON.stringify({{ status: "ERROR", msg: "No iframe found." }});
                                            let doc = iframe.contentWindow.document;

                                            let listLoaded = false;
                                            logs.push("Waiting for report list to load...");
                                            for (let i = 0; i < 20; i++) {{
                                                let divs = doc.querySelectorAll('div.fw-semibold');
                                                for (let d of divs) {{
                                                    let textLower = d.textContent.toLowerCase();
                                                    if (textLower.includes('report manager') || textLower.includes('report manger') || textLower.includes(reportType.toLowerCase()) || textLower.includes('enquiry') || textLower.includes('Standard Report'.toLowerCase())) {{
                                                        listLoaded = true; break;
                                                    }}
                                                }}
                                                if (listLoaded) break;
                                                await sleep(500);
                                            }}
                                            if (!listLoaded) return JSON.stringify({{ status: "ERROR", msg: "Report list timeout.", logs }});
                                            logs.push("Report list loaded.");

                                            await sleep(1000);
                                            let searchInputList = doc.querySelector('input[formcontrolname="searchInput"], input[placeholder="Search"]');
                                            if (searchInputList) {{
                                                logs.push("Found searchInputList");
                                                searchInputList.focus();
                                                searchInputList.value = reportName;
                                                searchInputList.dispatchEvent(new Event('input', {{ bubbles: true }}));
                                                searchInputList.dispatchEvent(new Event('change', {{ bubbles: true }}));
                                                searchInputList.dispatchEvent(new KeyboardEvent('keyup', {{ key: 'Enter', code: 'Enter', keyCode: 13, which: 13, bubbles: true }}));
                                            }} else {{
                                                logs.push("Warning: searchInputList not found.");
                                            }}

                                            let reportFound = false;
                                            logs.push("Waiting for report span in list: " + reportName);
                                            for (let i = 0; i < 20; i++) {{
                                                let itemXpath = `//li[contains(@class, 'sub-list-items')]//span[contains(text(), '${{reportName}}')]`;
                                                let itemResult = doc.evaluate(itemXpath, doc, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                                                let reportSpan = itemResult.singleNodeValue;
                                                if (reportSpan) {{
                                                    logs.push("Found reportSpan");
                                                    let liElement = reportSpan.closest('li.sub-list-items');
                                                    if (liElement) {{
                                                        liElement.click();
                                                        logs.push("Clicked liElement");
                                                    }} else {{
                                                        reportSpan.click();
                                                        logs.push("Clicked reportSpan");
                                                    }}
                                                    reportFound = true;
                                                    break;
                                                }}
                                                await sleep(1500);
                                            }}
                                            if (!reportFound) return JSON.stringify({{ status: "ERROR", msg: "Report name not found: " + reportName, logs }});
                                            return JSON.stringify({{ status: "SUCCESS", logs }});
                                        }})({}, {});
                                        "#,
                                        serde_json::to_string(&active_report_type).unwrap(),
                                        serde_json::to_string(&active_report_name).unwrap()
                                    );

                                    info!("Searching & Selecting Report: {}", active_report_name);
                                    if let Ok(res) = tab.evaluate(&step2_js, true) {
                                        if let Some(v) = res.value {
                                            if let Some(s) = v.as_str() {
                                                let parsed: serde_json::Value =
                                                    serde_json::from_str(s).unwrap_or_default();
                                                let status = parsed
                                                    .get("status")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("UNKNOWN");
                                                let logs = parsed
                                                    .get("logs")
                                                    .map(|v| v.to_string())
                                                    .unwrap_or_default();
                                                if status == "ERROR" {
                                                    let msg = parsed
                                                        .get("msg")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("Unknown error");
                                                    error!(
                                                        "Step 2 Failed: {} | Logs: {}",
                                                        msg, logs
                                                    );
                                                    return Err(anyhow::anyhow!(
                                                        "Automation Step 2 failed"
                                                    ));
                                                } else {
                                                    info!("Step 2 Success | Logs: {}", logs);
                                                }
                                            }
                                        }
                                    } else {
                                        error!("Failed to evaluate Step 2 JS.");
                                        return Err(anyhow::anyhow!("Automation Step 2 failed"));
                                    }
                                    crate::yasweb::browser::save_html_state(
                                        &tab,
                                        active_report_name,
                                        step_num,
                                        "After Selecting Report from List",
                                    );
                                    step_num += 1;

                                    // STEP 3: Wait for Binding
                                    let step3_js = format!(
                                        r#"
                                        (async function(reportName) {{
                                            function sleep(ms) {{ return new Promise(r => setTimeout(r, ms)); }}
                                            let logs = [];
                                            let doc = document.querySelector('iframe').contentWindow.document;
                                            let reportBound = false;
                                            logs.push("Waiting for report binding...");
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
                                            if (!reportBound) return JSON.stringify({{ status: "ERROR", msg: "Binding timeout.", logs }});
                                            return JSON.stringify({{ status: "SUCCESS", logs }});
                                        }})({});
                                        "#,
                                        serde_json::to_string(&active_report_name).unwrap()
                                    );

                                    info!("Waiting for Report Binding...");
                                    if let Ok(res) = tab.evaluate(&step3_js, true) {
                                        if let Some(v) = res.value {
                                            if let Some(s) = v.as_str() {
                                                let parsed: serde_json::Value =
                                                    serde_json::from_str(s).unwrap_or_default();
                                                let status = parsed
                                                    .get("status")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("UNKNOWN");
                                                let logs = parsed
                                                    .get("logs")
                                                    .map(|v| v.to_string())
                                                    .unwrap_or_default();
                                                if status == "ERROR" {
                                                    let msg = parsed
                                                        .get("msg")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("Unknown error");
                                                    error!(
                                                        "Step 3 Failed: {} | Logs: {}",
                                                        msg, logs
                                                    );
                                                    return Err(anyhow::anyhow!(
                                                        "Automation Step 3 failed"
                                                    ));
                                                } else {
                                                    info!("Step 3 Success | Logs: {}", logs);
                                                }
                                            }
                                        }
                                    } else {
                                        error!("Failed to evaluate Step 3 JS.");
                                        return Err(anyhow::anyhow!("Automation Step 3 failed"));
                                    }
                                    crate::yasweb::browser::save_html_state(
                                        &tab,
                                        active_report_name,
                                        step_num,
                                        "After Report Bound",
                                    );
                                    step_num += 1;

                                    // STEP 4: Fill Filters & Click Search

                                    let step4_fill_js = format!(
                                        r#"
                                        (async function(filters) {{
                                            function sleep(ms) {{ return new Promise(r => setTimeout(r, ms)); }}
                                            let logs = [];
                                            async function simulateTyping(inputElem, text) {{
                                                inputElem.focus();
                                                inputElem.value = '';
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

                                            let doc = document.querySelector('iframe').contentWindow.document;

                                            let labels = doc.querySelectorAll('mat-label');
                                            let discoveredFilters = [];
                                            for (let lbl of labels) {{
                                                if (lbl.innerText) {{ discoveredFilters.push(lbl.innerText.trim()); }}
                                            }}
                                            logs.push("Discovered filters count: " + discoveredFilters.length);

                                            for (const [key, value] of Object.entries(filters)) {{
                                                logs.push("Applying filter: " + key + " = " + value);
                                                for (let lbl of labels) {{
                                                    if (lbl.innerText.trim().toLowerCase() === key.toLowerCase()) {{
                                                        let labelParent = lbl.closest('label');
                                                        if (labelParent && labelParent.hasAttribute('for')) {{
                                                            let inputId = labelParent.getAttribute('for');
                                                            let input = doc.getElementById(inputId);
                                                            if (input) {{
                                                                if (input.tagName === 'INPUT') {{
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
                                                                    logs.push("Typed into INPUT for " + key);
                                                                    break;
                                                                }} else if (input.tagName === 'MAT-SELECT') {{
                                                                    input.click();
                                                                    await sleep(500);

                                                                    let options = doc.querySelectorAll('mat-option');
                                                                    if (options.length === 0) {{
                                                                        let mainOverlay = document.querySelector('.cdk-overlay-container');
                                                                        if (mainOverlay) {{
                                                                            options = mainOverlay.querySelectorAll('mat-option');
                                                                        }}
                                                                    }}

                                                                    let optionFound = false;
                                                                    for (let opt of options) {{
                                                                        if (opt.textContent.toLowerCase().includes(value.toLowerCase())) {{
                                                                            opt.click();
                                                                            optionFound = true;
                                                                            logs.push("Selected MAT-OPTION for " + key);
                                                                            break;
                                                                        }}
                                                                    }}

                                                                    if (!optionFound) {{
                                                                        let backdrop = doc.querySelector('.cdk-overlay-backdrop') || document.querySelector('.cdk-overlay-backdrop');
                                                                        if (backdrop) backdrop.click();
                                                                        logs.push("MAT-OPTION not found for " + key);
                                                                    }}

                                                                    await sleep(500);
                                                                    break;
                                                                }}
                                                            }}
                                                        }}
                                                    }}
                                                }}
                                            }}
                                            return JSON.stringify({{ status: "SUCCESS", discovered: discoveredFilters, logs }});
                                        }})({});
                                        "#,
                                        filters_json
                                    );

                                    info!("Filling filters...");
                                    if let Ok(res) = tab.evaluate(&step4_fill_js, true) {
                                        if let Some(v) = res.value {
                                            if let Some(s) = v.as_str() {
                                                let parsed: serde_json::Value =
                                                    serde_json::from_str(s).unwrap_or_default();
                                                let status = parsed
                                                    .get("status")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("UNKNOWN");
                                                let logs = parsed
                                                    .get("logs")
                                                    .map(|v| v.to_string())
                                                    .unwrap_or_default();

                                                if status == "ERROR" {
                                                    let msg = parsed
                                                        .get("msg")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("Unknown error");
                                                    error!("Step 4 (Fill Filters) Failed: {} | Logs: {}", msg, logs);
                                                    return Err(anyhow::anyhow!(
                                                        "Automation Step 4 (Fill) failed"
                                                    ));
                                                } else {
                                                    info!(
                                                        "Step 4 (Fill Filters) Success | Logs: {}",
                                                        logs
                                                    );
                                                    if let Some(arr) = parsed
                                                        .get("discovered")
                                                        .and_then(|a| a.as_array())
                                                    {
                                                        for item in arr {
                                                            if let Some(val) = item.as_str() {
                                                                discovered_filters
                                                                    .push(val.to_string());
                                                            }
                                                        }
                                                        info!(
                                                            "Discovered filters: {:?}",
                                                            discovered_filters
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        error!("Failed to evaluate Step 4 (Fill) JS.");
                                        return Err(anyhow::anyhow!(
                                            "Automation Step 4 (Fill) failed"
                                        ));
                                    }
                                    crate::yasweb::browser::save_html_state(
                                        &tab,
                                        active_report_name,
                                        step_num,
                                        "After filling in the filters",
                                    );
                                    step_num += 1;

                                    let step4_search_js = r#"
                                        (async function() {
                                            function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }
                                            let logs = [];
                                            let doc = document.querySelector('iframe').contentWindow.document;

                                            logs.push("Waiting for Search button to appear...");
                                            let clickedSearch = false;

                                            for (let i = 0; i < 20; i++) {
                                                // Try mattooltip="Search"
                                                let btn = doc.querySelector('button[mattooltip="Search"]');
                                                if (btn && btn.offsetParent !== null) {
                                                    btn.click();
                                                    clickedSearch = true;
                                                    logs.push("Clicked button[mattooltip='Search']");
                                                    break;
                                                }

                                                // Try bi-search icon
                                                let searchBtnIcon = doc.querySelector('i.bi-search');
                                                if (searchBtnIcon && searchBtnIcon.offsetParent !== null) {
                                                    let parentBtn = searchBtnIcon.closest('button');
                                                    if (parentBtn) {
                                                        parentBtn.click();
                                                        clickedSearch = true;
                                                        logs.push("Clicked button containing i.bi-search");
                                                        break;
                                                    }
                                                }

                                                await sleep(1000);
                                            }

                                            if (!clickedSearch) {
                                                return JSON.stringify({ status: "ERROR", msg: "Search button not found.", logs });
                                            }

                                            return JSON.stringify({ status: "SUCCESS", logs });
                                        })();
                                    "#;
                                    info!("Clicking Search...");
                                    if let Ok(res) = tab.evaluate(step4_search_js, true) {
                                        if let Some(v) = res.value {
                                            if let Some(s) = v.as_str() {
                                                let parsed: serde_json::Value =
                                                    serde_json::from_str(s).unwrap_or_default();
                                                let status = parsed
                                                    .get("status")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("UNKNOWN");
                                                let logs = parsed
                                                    .get("logs")
                                                    .map(|v| v.to_string())
                                                    .unwrap_or_default();

                                                if status == "ERROR" {
                                                    let msg = parsed
                                                        .get("msg")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("Unknown error");
                                                    error!(
                                                        "Step 4 (Search) Failed: {} | Logs: {}",
                                                        msg, logs
                                                    );
                                                    return Err(anyhow::anyhow!(
                                                        "Automation Step 4 (Search) failed"
                                                    ));
                                                } else {
                                                    info!(
                                                        "Step 4 (Search) Success | Logs: {}",
                                                        logs
                                                    );
                                                }
                                            }
                                        }
                                    } else {
                                        error!("Failed to evaluate Step 4 (Search) JS.");
                                        return Err(anyhow::anyhow!(
                                            "Automation Step 4 (Search) failed"
                                        ));
                                    }
                                    crate::yasweb::browser::save_html_state(
                                        &tab,
                                        active_report_name,
                                        step_num,
                                        "After clicking the Search button",
                                    );
                                    step_num += 1;

                                    // STEP 5: Poll for loader across BOTH documents
                                    info!(
                                        "Waiting for loaders to clear (Max timeout loops: {})...",
                                        timeout_loops
                                    );
                                    let mut loader_found_at_least_once = false;
                                    let check_loader_js = r#"
                                        (function() {
                                            let hasLoader = false;

                                            // Check main document
                                            let mainLoader = document.querySelector('.loading-screen-wrapper, mat-progress-bar, .dx-loadpanel-content');
                                            if (mainLoader && mainLoader.offsetParent !== null) hasLoader = true;

                                            // Check iframe document
                                            try {
                                                let iframe = document.querySelector('iframe');
                                                if (iframe && iframe.contentWindow && iframe.contentWindow.document) {
                                                    let doc = iframe.contentWindow.document;
                                                    let iLoader = doc.querySelector('.loading-screen-wrapper, mat-progress-bar, .dx-loadpanel-content');
                                                    if (iLoader && iLoader.offsetParent !== null) hasLoader = true;
                                                }
                                            } catch(e) {}

                                            return hasLoader;
                                        })();
                                    "#;

                                    // Fast initial wait for loader to potentially appear
                                    for _ in 0..10 {
                                        if let Ok(res) = tab.evaluate(check_loader_js, true) {
                                            if let Some(v) = res.value {
                                                if v.as_bool().unwrap_or(false) {
                                                    loader_found_at_least_once = true;
                                                    break;
                                                }
                                            }
                                        }
                                        std::thread::sleep(Duration::from_millis(500));
                                    }

                                    if loader_found_at_least_once {
                                        info!("Loader detected! Waiting for it to disappear...");
                                        for i in 0..timeout_loops {
                                            if let Ok(res) = tab.evaluate(check_loader_js, true) {
                                                if let Some(v) = res.value {
                                                    if !v.as_bool().unwrap_or(false) {
                                                        info!("Loader cleared after {} loops.", i);
                                                        break;
                                                    }
                                                }
                                            }
                                            std::thread::sleep(Duration::from_secs(10));
                                            crate::yasweb::browser::save_html_state(
                                                &tab,
                                                active_report_name,
                                                step_num,
                                                &format!("Waiting for loader (Loop {})", i),
                                            );
                                            step_num += 1;
                                        }
                                    } else {
                                        info!("No loader detected after search click.");
                                    }
                                    crate::yasweb::browser::save_html_state(
                                        &tab,
                                        active_report_name,
                                        step_num,
                                        "After Loader Clear",
                                    );
                                    step_num += 1;
                                    std::thread::sleep(Duration::from_secs(2));

                                    // STEP 6: Export & XLSX

                                    // STEP 6: Export & XLSX
                                    let step6_export_js = r#"
                                        (async function() {
                                            let doc = document.querySelector('iframe').contentWindow.document;
                                            let logs = [];

                                            let exportBtn = doc.querySelector('div[aria-label="Export"]');
                                            if (exportBtn && exportBtn.offsetParent !== null) {
                                                exportBtn.click();
                                                logs.push("Clicked div[aria-label='Export']");
                                                return JSON.stringify({ status: "SUCCESS", logs });
                                            }

                                            let dxButtons = doc.querySelectorAll('.dx-button-text');
                                            for (let btn of dxButtons) {
                                                if (btn.textContent.trim() === 'Export') { exportBtn = btn.closest('div[role="button"]'); break; }
                                            }
                                            if (!exportBtn) {
                                                let allButtons = doc.querySelectorAll('button, div[role="button"], span');
                                                for (let btn of allButtons) {
                                                    if (btn.textContent.trim() === 'Export' && btn.offsetParent !== null) {
                                                        exportBtn = btn; break;
                                                    }
                                                }
                                            }

                                            if (!exportBtn) return JSON.stringify({ status: "ERROR", msg: "Export button not found.", logs });

                                            exportBtn.click();
                                            logs.push("Clicked Export button via fallback");
                                            return JSON.stringify({ status: "SUCCESS", logs });
                                        })();
                                    "#;

                                    info!("Clicking Export...");
                                    if let Ok(res) = tab.evaluate(step6_export_js, true) {
                                        if let Some(v) = res.value {
                                            if let Some(s) = v.as_str() {
                                                let parsed: serde_json::Value =
                                                    serde_json::from_str(s).unwrap_or_default();
                                                let status = parsed
                                                    .get("status")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("UNKNOWN");
                                                let logs = parsed
                                                    .get("logs")
                                                    .map(|v| v.to_string())
                                                    .unwrap_or_default();

                                                if status == "ERROR" {
                                                    let msg = parsed
                                                        .get("msg")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("Unknown error");
                                                    error!(
                                                        "Step 6 (Export) Failed: {} | Logs: {}",
                                                        msg, logs
                                                    );
                                                    return Err(anyhow::anyhow!(
                                                        "Automation Step 6 (Export) failed"
                                                    ));
                                                } else {
                                                    info!(
                                                        "Step 6 (Export) Success | Logs: {}",
                                                        logs
                                                    );
                                                }
                                            }
                                        }
                                    } else {
                                        error!("Failed to evaluate Step 6 (Export) JS.");
                                        return Err(anyhow::anyhow!(
                                            "Automation Step 6 (Export) failed"
                                        ));
                                    }
                                    std::thread::sleep(Duration::from_secs(1));
                                    crate::yasweb::browser::save_html_state(
                                        &tab,
                                        active_report_name,
                                        step_num,
                                        "After clicking Export",
                                    );
                                    step_num += 1;

                                    let step6_xlsx_js = r#"
                                        (async function() {
                                            let doc = document.querySelector('iframe').contentWindow.document;
                                            let logs = [];
                                            let xlsxOption = null;
                                            let listItems = doc.querySelectorAll('.dx-list-item-content');
                                            for (let item of listItems) {
                                                if (item.textContent.trim() === 'XLSX') { xlsxOption = item.closest('.dx-list-item'); break; }
                                            }
                                            if (!xlsxOption) {
                                                let allSpans = doc.querySelectorAll('span, div');
                                                for (let span of allSpans) {
                                                    if (span.textContent.trim() === 'XLSX' && span.offsetParent !== null) {
                                                        xlsxOption = span; break;
                                                    }
                                                }
                                            }

                                            if (!xlsxOption) return JSON.stringify({ status: "ERROR", msg: "XLSX option not found.", logs });
                                            xlsxOption.click();
                                            logs.push("Clicked XLSX option");
                                            return JSON.stringify({ status: "SUCCESS", logs });
                                        })();
                                    "#;

                                    info!("Clicking XLSX...");
                                    if let Ok(res) = tab.evaluate(step6_xlsx_js, true) {
                                        if let Some(v) = res.value {
                                            if let Some(s) = v.as_str() {
                                                let parsed: serde_json::Value =
                                                    serde_json::from_str(s).unwrap_or_default();
                                                let status = parsed
                                                    .get("status")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("UNKNOWN");
                                                let logs = parsed
                                                    .get("logs")
                                                    .map(|v| v.to_string())
                                                    .unwrap_or_default();

                                                if status == "ERROR" {
                                                    let msg = parsed
                                                        .get("msg")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("Unknown error");
                                                    error!(
                                                        "Step 6 (XLSX) Failed: {} | Logs: {}",
                                                        msg, logs
                                                    );
                                                    return Err(anyhow::anyhow!(
                                                        "Automation Step 6 (XLSX) failed"
                                                    ));
                                                } else {
                                                    info!("Step 6 (XLSX) Success | Logs: {}", logs);
                                                    info!("JS Automation Sequence Completed Successfully!");
                                                }
                                            }
                                        }
                                    } else {
                                        error!("Failed to evaluate Step 6 (XLSX) JS.");
                                        return Err(anyhow::anyhow!(
                                            "Automation Step 6 (XLSX) failed"
                                        ));
                                    }
                                    crate::yasweb::browser::save_html_state(
                                        &tab,
                                        active_report_name,
                                        step_num,
                                        "After clicking XLSX",
                                    );
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
        let timeout_seconds = config.timeout_minutes * 60;

        for _ in 0..timeout_seconds {
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

    Ok(discovered_filters)
}
