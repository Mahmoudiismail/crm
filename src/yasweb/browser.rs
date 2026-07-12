use anyhow::{Context, Result};
use headless_chrome::{Browser, protocol::cdp::types::Event};
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

pub fn run_browser_tab(
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
                                                    if (textLower.includes('report manager') || textLower.includes('report manger') || textLower.includes(reportType.toLowerCase()) || textLower.includes('enquiry') || textLower.includes('Standard Report'.toLowerCase())) {{
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
