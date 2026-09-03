use anyhow::Result;
use headless_chrome::Tab;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::yasweb::browser::{debug, javascript};

pub fn navigate_and_run_report(
    tab: &Arc<Tab>,
    active_report_name: &str,
    active_report_type: &str,
    active_filters: &HashMap<String, String>,
    _timeout_minutes: u64,
    step_num: &mut u32,
) -> Result<Vec<String>> {
    let mut discovered_filters = Vec::new();

    // Replicate navigation block to find "MIS" and run steps
    info!("Waiting for #menuPinnedBtn...");
    match tab.wait_for_element("#menuPinnedBtn:not(.d-none), #pinButton:not(.d-none)") {
        Ok(menu_btn) => {
            let mut mis_found = false;
            let mis_selector = ".misManagement";

            for attempt in 1..=3 {
                let mut sidebar_toggled = false;

                info!("Clicking #menuPinnedBtn (Attempt {})...", attempt);
                if let Err(e) = menu_btn.click() {
                    error!("Failed to click menu button: {:?}", e);
                    if let Ok(html) = tab.get_content() {
                        error!("Page HTML after failed menu click:\n{}", html);
                    }
                } else {
                    for _ in 0..10 {
                        let check_js = "document.querySelector('.menuModules') ? document.querySelector('.menuModules').classList.contains('show-modules') : false";
                        if let Ok(eval_result) = tab.evaluate(check_js, true) {
                            if let Some(val) = eval_result.value {
                                if val.as_bool().unwrap_or(false) {
                                    sidebar_toggled = true;
                                    break;
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
                    debug::save_html_state(
                        tab,
                        active_report_name,
                        *step_num,
                        "After clicking #menuPinnedBtn",
                    );
                    *step_num += 1;

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

            if !mis_found {
                error!(
                    "MIS module ('{}') not found after all attempts.",
                    mis_selector
                );
                if let Ok(html) = tab.get_content() {
                    error!("Page HTML at MIS module wait timeout:\n{}", html);
                }
                return Err(anyhow::anyhow!("MIS module wait timeout"));
            }

            match tab.wait_for_element(mis_selector) {
                Ok(mis_module) => {
                    info!("Clicking on MIS module...");
                    if let Err(e) = mis_module.click() {
                        error!("Failed to click MIS module: {:?}", e);
                        if let Ok(html) = tab.get_content() {
                            error!("Page HTML after failed MIS module click:\n{}", html);
                        }
                    } else {
                        info!("Clicked MIS successfully. Waiting for MIS Reports button...");
                        tracing::debug!("Waiting for MIS Reports button...");
                        if let Ok(html) = tab.get_content() {
                            tracing::trace!(
                                "Page HTML immediately after clicking MIS module:\n{}",
                                html
                            );
                        }

                        let mut mis_reports_found = false;
                        let mis_reports_xpath = "//div[contains(@class, 'label') and contains(@class, 'fw-bold') and contains(text(), 'MIS Reports')]";

                        for _ in 0..10 {
                            if tab.find_element_by_xpath(mis_reports_xpath).is_ok() {
                                mis_reports_found = true;
                                break;
                            }
                            std::thread::sleep(Duration::from_secs(2));
                        }

                        if !mis_reports_found {
                            error!("MIS Reports button not found after wait.");
                            if let Ok(html) = tab.get_content() {
                                error!("Page HTML at MIS Reports button wait timeout:\n{}", html);
                            }
                            return Err(anyhow::anyhow!("MIS Reports button wait timeout"));
                        }

                        info!("MIS Reports button successfully verified.");
                        println!("MIS Reports button successfully verified.");
                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After clicking MIS module",
                        );
                        *step_num += 1;

                        std::thread::sleep(Duration::from_secs(2));

                        info!("Searching for auto-login iframe...");
                        let mut iframe_node_id = None;

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
                        // let timeout_loops = (timeout_minutes * 60) / 10;

                        // STEP 1: Select Report Type
                        tracing::debug!(
                            "Executing JavaScript to select report type: {}",
                            active_report_type
                        );
                        let step1_js = format!(
                            r#"
                                        (async function(reportType) {{
                                            function sleep(ms) {{ return new Promise(r => setTimeout(r, ms)); }}
                                            let logs = [];
                                            let iframe = document.querySelector('iframe');
                                            if (!iframe) return JSON.stringify({{ status: "ERROR", msg: "No iframe found.", logs }});
                                            let doc = iframe.contentWindow.document;

                                            let clickedType = false;
                                            logs.push("Searching for reportType: " + reportType);

                                            // Ensure the iframe has actually loaded some content
                                            for(let i=0; i<30; i++) {{
                                                if (doc.body && doc.body.innerHTML.trim().length > 0) break;
                                                await sleep(500);
                                            }}

                                            for (let i = 0; i < 20; i++) {{
                                                let xpathType = `//*[contains(normalize-space(.), '${{reportType}}')]/ancestor-or-self::mat-radio-button`;
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
                                                    let fallbackXpath = `//label[contains(normalize-space(.), '${{reportType}}')]`;
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

                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "Before selecting Report Type",
                        );
                        *step_num += 1;
                        info!("Selecting Report Type: {}", active_report_type);
                        javascript::evaluate_automation_step(
                            tab,
                            &step1_js,
                            "Step 1 (Report Type)",
                        )?;
                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After Selecting Report Type",
                        );
                        *step_num += 1;

                        // STEP 1.5 & 2: Wait for List, Search, and Select Report
                        tracing::debug!(
                            "Executing JavaScript to search & select report: {}",
                            active_report_name
                        );
                        let step2_js = format!(
                            r#"
                                        (async function(reportType, reportName) {{
                                            function sleep(ms) {{ return new Promise(r => setTimeout(r, ms)); }}
                                            let logs = [];
                                            let iframe = document.querySelector('iframe');
                                            if (!iframe) return JSON.stringify({{ status: "ERROR", msg: "No iframe found.", logs }});
                                            let doc = iframe.contentWindow.document;

                                            logs.push("Waiting for loader to disappear before typing...");
                                            for(let i=0; i<30; i++) {{
                                                let loader = document.querySelector('.loading-screen-wrapper, mat-progress-bar, .dx-loadpanel') || doc.querySelector('.loading-screen-wrapper, mat-progress-bar, .dx-loadpanel');
                                                let isLoaderVisible = false;
                                                if (loader) {{
                                                    let style = loader.ownerDocument.defaultView.getComputedStyle(loader);
                                                    if (style.display !== 'none' && style.opacity !== '0' && style.visibility !== 'hidden') {{
                                                        isLoaderVisible = true;
                                                    }}
                                                }}
                                                if (!isLoaderVisible) {{
                                                    break;
                                                }}
                                                await sleep(500);
                                            }}

                                            let listLoaded = false;
                                            logs.push("Waiting for report list to load (tree-view)...");
                                            for (let i = 0; i < 30; i++) {{
                                                if (doc.querySelectorAll('.tree-view').length > 0) {{
                                                    listLoaded = true; break;
                                                }}
                                                await sleep(500);
                                            }}
                                            if (!listLoaded) return JSON.stringify({{ status: "ERROR", msg: "Report list timeout.", logs }});
                                            logs.push("Report list loaded.");
                                            await sleep(1000);

                                            logs.push("Typing in search input...");
                                            let searchInputList = doc.querySelector('input[formcontrolname="searchInput"], input[placeholder="Search"]');
                                            if (searchInputList) {{
                                                searchInputList.focus();
                                                searchInputList.value = '';
                                                for (let i = 0; i < reportName.length; i++) {{
                                                    searchInputList.value += reportName[i];
                                                    searchInputList.dispatchEvent(new Event('input', {{ bubbles: true }}));
                                                    await sleep(5);
                                                }}
                                                searchInputList.dispatchEvent(new Event('change', {{ bubbles: true }}));

                                                // Trigger the 'Enter' key to execute the search
                                                searchInputList.dispatchEvent(new KeyboardEvent('keydown', {{ bubbles: true, cancelable: true, key: 'Enter', code: 'Enter', keyCode: 13 }}));
                                                searchInputList.dispatchEvent(new KeyboardEvent('keyup', {{ bubbles: true, cancelable: true, key: 'Enter', code: 'Enter', keyCode: 13 }}));

                                                searchInputList.blur();
                                                searchInputList.dispatchEvent(new Event('blur', {{ bubbles: true }}));
                                                logs.push("Typed search input using simulation and triggered Enter");
                                                await sleep(500); // Wait after typing
                                            }} else {{
                                                logs.push("Warning: searchInputList not found.");
                                            }}

                                            let reportFound = false;
                                            logs.push("Waiting for report span in list: " + reportName);
                                            for (let i = 0; i < 30; i++) {{
                                                let exactMatchSpan = null;
                                                let partialMatchSpan = null;

                                                let spans = doc.querySelectorAll('.tree-view span');
                                                let searchName = reportName.trim().toLowerCase();

                                                for (let span of spans) {{
                                                    let spanText = span.textContent.trim().toLowerCase();
                                                    if (spanText === searchName) {{
                                                        exactMatchSpan = span;
                                                        break;
                                                    }} else if (!partialMatchSpan && spanText.includes(searchName)) {{
                                                        partialMatchSpan = span;
                                                    }}
                                                }}

                                                let bestMatchSpan = exactMatchSpan || partialMatchSpan;

                                                if (bestMatchSpan) {{
                                                    if (exactMatchSpan) {{
                                                        logs.push("Found exact match reportSpan");
                                                    }} else {{
                                                        logs.push("Found partial match reportSpan");
                                                    }}

                                                    let listItemElement = bestMatchSpan.closest('.sub-list-items');
                                                    if (listItemElement) {{
                                                        listItemElement.click();
                                                        logs.push("Clicked sub-list-items");
                                                    }} else {{
                                                        bestMatchSpan.click();
                                                        logs.push("Clicked reportSpan");
                                                    }}
                                                    reportFound = true;
                                                    break;
                                                }}
                                                await sleep(1000);
                                            }}
                                            if (!reportFound) return JSON.stringify({{ status: "ERROR", msg: "Report name not found: " + reportName, logs }});
                                            return JSON.stringify({{ status: "SUCCESS", logs }});
                                        }})({}, {});
                                        "#,
                            serde_json::to_string(&active_report_type).unwrap(),
                            serde_json::to_string(&active_report_name).unwrap()
                        );

                        info!("Searching & Selecting Report: {}", active_report_name);
                        javascript::evaluate_automation_step(
                            tab,
                            &step2_js,
                            "Step 2 (Select Report)",
                        )?;
                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After Selecting Report from List",
                        );
                        *step_num += 1;

                        // STEP 3: Wait for binding
                        tracing::debug!("Executing JavaScript to wait for binding...");
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
                                                if (!reportBound) {{
                                                    let headers = doc.querySelectorAll('.fw-semibold, .fw-bold');
                                                    for (let h of headers) {{
                                                        if (h.innerText.includes(reportName) || h.textContent.includes(reportName)) {{
                                                            reportBound = true; break;
                                                        }}
                                                    }}
                                                }}
                                                if (reportBound) break;
                                                await sleep(1000);
                                            }}
                                            if (!reportBound) return JSON.stringify({{ status: "ERROR", msg: "Binding timeout.", logs }});
                                            return JSON.stringify({{ status: "SUCCESS", logs }});
                                        }})({});
                                        "#,
                            serde_json::to_string(&active_report_name).unwrap()
                        );

                        info!("Waiting for Report Binding...");
                        javascript::evaluate_automation_step(tab, &step3_js, "Step 3 (Binding)")?;
                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After Report Bound",
                        );
                        *step_num += 1;

                        // STEP 3.5 & 4: Wait for Loader to disappear, then fill filters
                        tracing::debug!("Executing JavaScript to apply filters...");
                        let step4_fill_js = format!(
                            r#"
                                        (async function(filters) {{
                                            function sleep(ms) {{ return new Promise(r => setTimeout(r, ms)); }}
                                            let logs = [];
                                            let iframe = document.querySelector('iframe');
                                            let doc = iframe.contentWindow.document;

                                            logs.push("Waiting for loader to disappear...");
                                            let loaderGone = false;
                                            for(let i=0; i<50; i++) {{
                                                let loader = document.querySelector('.loading-screen-wrapper, mat-progress-bar, .dx-loadpanel') || doc.querySelector('.loading-screen-wrapper, mat-progress-bar, .dx-loadpanel');
                                                let isLoaderVisible = false;
                                                if (loader) {{
                                                    let style = loader.ownerDocument.defaultView.getComputedStyle(loader);
                                                    if (style.display !== 'none' && style.opacity !== '0' && style.visibility !== 'hidden') {{
                                                        isLoaderVisible = true;
                                                    }}
                                                }}
                                                if (!isLoaderVisible && doc.querySelectorAll('mat-label').length > 0) {{
                                                    loaderGone = true;
                                                    break;
                                                }}
                                                await sleep(500);
                                            }}
                                            if (!loaderGone) logs.push("Warning: Loader timeout or labels never appeared.");

                                            async function simulateTyping(inputElem, text) {{
                                                inputElem.focus();
                                                inputElem.value = '';
                                                for (let i = 0; i < text.length; i++) {{
                                                    inputElem.value += text[i];
                                                    inputElem.dispatchEvent(new Event('input', {{ bubbles: true }}));
                                                    await sleep(10);
                                                }}
                                                inputElem.dispatchEvent(new Event('change', {{ bubbles: true }}));
                                                inputElem.blur();
                                                inputElem.dispatchEvent(new Event('blur', {{ bubbles: true }}));
                                            }}

                                            let labels = doc.querySelectorAll('mat-label');
                                            let discoveredFilters = [];
                                            for (let lbl of labels) {{
                                                if (lbl.innerText) {{ discoveredFilters.push(lbl.innerText.trim()); }}
                                            }}
                                            logs.push("Discovered filters count: " + discoveredFilters.length);

                                            for (const [key, value] of Object.entries(filters)) {{
                                                if (!value || value.trim() === '') {{
                                                    logs.push("Skipping empty filter: " + key);
                                                    continue;
                                                }}
                                                logs.push("Applying filter: " + key + " = " + value);
                                                let normalizedKey = key.toLowerCase().replace(/_/g, ' ');
                                                let filterFilled = false;

                                                for (let lbl of labels) {{
                                                    let labelText = lbl.innerText.trim().toLowerCase().replace(/_/g, ' ');
                                                    if (labelText === normalizedKey) {{
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
                                                                            v = parts[0].padStart(2, '0') + "-" + parts[1].padStart(2, '0') + "-" + parts[2] + (v.includes(' ') ? ' ' + v.split(' ').slice(1).join(' ') : '');
                                                                        }}
                                                                    }}
                                                                    await simulateTyping(input, v);
                                                                    logs.push("Typed into INPUT for " + key);
                                                                    filterFilled = true;
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
                                                                    filterFilled = true;
                                                                    break;
                                                                }}
                                                            }}
                                                        }}
                                                    }}
                                                }}
                                                if (!filterFilled) logs.push("Failed to fill filter: " + key);
                                            }}
                                            return JSON.stringify({{ status: "SUCCESS", discovered_filters: discoveredFilters, logs }});
                                        }})({});
                                        "#,
                            filters_json
                        );

                        info!("Applying Filters...");
                        let step4_res = javascript::evaluate_automation_step(
                            tab,
                            &step4_fill_js,
                            "Step 4 (Fill Filters)",
                        )?;
                        if let Some(arr) = step4_res
                            .get("discovered_filters")
                            .and_then(|v| v.as_array())
                        {
                            for f in arr {
                                if let Some(s) = f.as_str() {
                                    discovered_filters.push(s.to_string());
                                }
                            }
                        }

                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After Filters Applied",
                        );
                        *step_num += 1;

                        std::thread::sleep(Duration::from_secs(1));

                        // STEP 5: Search Click
                        tracing::debug!("Executing JavaScript to click search...");
                        let step5_search_js = r#"
                                        (async function() {
                                            function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }
                                            let logs = [];
                                            let doc = document.querySelector('iframe').contentWindow.document;

                                            logs.push("Waiting for Search button to appear...");
                                            let clickedSearch = false;

                                            for (let i = 0; i < 20; i++) {
                                                let btn = doc.querySelector('button[mattooltip="Search"]');
                                                if (btn && btn.offsetParent !== null) {
                                                    btn.click();
                                                    clickedSearch = true;
                                                    logs.push("Clicked button[mattooltip='Search']");
                                                    break;
                                                }

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
                        javascript::evaluate_automation_step(
                            tab,
                            step5_search_js,
                            "Step 5 (Search Click)",
                        )?;

                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After Clicking Search",
                        );
                        *step_num += 1;

                        // STEP 6: Wait for Loader and Click Export
                        tracing::debug!("Executing JavaScript to wait for export generation...");
                        let step6_export_js = r#"
                                        (async function() {
                                            function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }
                                            let logs = [];
                                            let doc = document.querySelector('iframe').contentWindow.document;

                                            logs.push("Waiting for report generation loader to finish...");
                                            let reportGenerated = false;
                                            for(let i=0; i<120; i++) {
                                                let loader = document.querySelector('.loading-screen-wrapper, mat-progress-bar, .dx-loadpanel') || doc.querySelector('.loading-screen-wrapper, mat-progress-bar, .dx-loadpanel');
                                                let isLoaderVisible = false;
                                                if (loader) {
                                                    let style = loader.ownerDocument.defaultView.getComputedStyle(loader);
                                                    if (style.display !== 'none' && style.opacity !== '0' && style.visibility !== 'hidden') {
                                                        isLoaderVisible = true;
                                                    }
                                                }
                                                if (!isLoaderVisible) {
                                                    reportGenerated = true;
                                                    break;
                                                }
                                                await sleep(1000);
                                                if (i % 5 === 0) logs.push("Still loading... " + (i*1) + "s elapsed");
                                            }
                                            if (!reportGenerated) return JSON.stringify({ status: "ERROR", msg: "Report generation timeout.", logs });

                                            await sleep(1500); // UI breathing room

                                            logs.push("Looking for Export button...");
                                            let exportBtn = null;
                                            for(let i=0; i<15; i++) {
                                                exportBtn = doc.querySelector('div[aria-label="Export"]');
                                                if (exportBtn && exportBtn.offsetParent !== null) break;

                                                let dxButtons = doc.querySelectorAll('.dx-button-text');
                                                for (let b of dxButtons) {
                                                    if (b.textContent.trim() === 'Export') {
                                                        exportBtn = b.closest('div[role="button"]');
                                                        break;
                                                    }
                                                }
                                                if (exportBtn && exportBtn.offsetParent !== null) break;

                                                let allButtons = doc.querySelectorAll('button, div[role="button"], span');
                                                for (let b of allButtons) {
                                                    if (b.textContent.trim() === 'Export' && b.offsetParent !== null) {
                                                        exportBtn = b;
                                                        break;
                                                    }
                                                }
                                                if (exportBtn && exportBtn.offsetParent !== null) break;
                                                await sleep(1000);
                                            }

                                            if (!exportBtn) return JSON.stringify({ status: "ERROR", msg: "Export button not found.", logs });

                                            exportBtn.click();
                                            logs.push("Clicked Export button");
                                            return JSON.stringify({ status: "SUCCESS", logs });
                                        })();
                                    "#;

                        info!("Waiting for generation & clicking Export...");
                        javascript::evaluate_automation_step(
                            tab,
                            step6_export_js,
                            "Step 6 (Wait & Export)",
                        )?;

                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After clicking Export",
                        );
                        *step_num += 1;

                        // STEP 7: Click XLSX
                        tracing::debug!("Executing JavaScript to click XLSX export option...");
                        let step7_xlsx_js = r#"
                                        (async function() {
                                            function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }
                                            let doc = document.querySelector('iframe').contentWindow.document;
                                            let mainDoc = document;
                                            let logs = [];
                                            let xlsxOption = null;

                                            for(let i=0; i<15; i++) {
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

                                                if (!xlsxOption) {
                                                    listItems = mainDoc.querySelectorAll('.dx-list-item-content');
                                                    for (let item of listItems) {
                                                        if (item.textContent.trim() === 'XLSX') { xlsxOption = item.closest('.dx-list-item'); break; }
                                                    }
                                                }
                                                if (!xlsxOption) {
                                                    let allSpans = mainDoc.querySelectorAll('span, div');
                                                    for (let span of allSpans) {
                                                        if (span.textContent.trim() === 'XLSX' && span.offsetParent !== null) {
                                                            xlsxOption = span; break;
                                                        }
                                                    }
                                                }

                                                if (xlsxOption) break;
                                                await sleep(1000);
                                            }

                                            if (!xlsxOption) return JSON.stringify({ status: "ERROR", msg: "XLSX option not found.", logs });
                                            xlsxOption.click();
                                            logs.push("Clicked XLSX option");
                                            return JSON.stringify({ status: "SUCCESS", logs });
                                        })();
                                    "#;

                        info!("Clicking XLSX...");
                        javascript::evaluate_automation_step(tab, step7_xlsx_js, "Step 7 (XLSX)")?;
                        info!("JS Automation Sequence Completed Successfully!");

                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
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
        Err(e) => {
            error!("Failed to find #menuPinnedBtn: {:?}", e);
            if let Ok(html) = tab.get_content() {
                error!("Page HTML at failure to find #menuPinnedBtn:\n{}", html);
            }
        }
    }

    Ok(discovered_filters)
}
