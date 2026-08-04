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
    timeout_minutes: u64,
    step_num: &mut u32,
) -> Result<Vec<String>> {
    let mut discovered_filters = Vec::new();

    // Replicate navigation block to find "MIS" and run steps
    info!("Waiting for #menuPinnedBtn...");
    match tab.wait_for_element("#menuPinnedBtn") {
        Ok(menu_btn) => {
            let mut mis_found = false;
            let mis_selector = "#misFocus";

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
                error!("MIS module not found after all attempts.");
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
                        let timeout_loops = (timeout_minutes * 60) / 10;

                        // STEP 1
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

                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "Before selecting Report Type",
                        );
                        *step_num += 1;
                        info!("Selecting Report Type: {}", active_report_type);
                        info!("Selecting Report Type...");
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

                        // STEP 2
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
                        info!("Selecting Report from List...");
                        javascript::evaluate_automation_step(tab, &step2_js, "Step 2")?;
                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After Selecting Report from List",
                        );
                        *step_num += 1;

                        // STEP 3
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
                        javascript::evaluate_automation_step(tab, &step3_js, "Step 3")?;
                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After Report Bound",
                        );
                        *step_num += 1;

                        // STEP 4
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
                        javascript::evaluate_automation_step(
                            tab,
                            step4_search_js,
                            "Step 4 (Search Click)",
                        )?;
                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After Clicking Search",
                        );
                        *step_num += 1;

                        // STEP 5: Wait for Loader
                        info!("Waiting for Report Loader to disappear... this may take a while depending on date range.");
                        let check_loader_js = r#"
                            (function() {
                                let doc = document.querySelector('iframe').contentWindow.document;
                                let loader = doc.querySelector('.dx-loadpanel');
                                if (loader) {
                                    let style = window.getComputedStyle(loader);
                                    if (style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0') {
                                        return true; // Still loading
                                    }
                                }
                                return false; // Not loading
                            })();
                        "#;
                        let mut was_loading = false;
                        for i in 0..timeout_loops {
                            std::thread::sleep(Duration::from_secs(10));
                            if let Ok(res) = tab.evaluate(check_loader_js, true) {
                                if let Some(val) = res.value {
                                    let is_loading = val.as_bool().unwrap_or(false);
                                    if is_loading {
                                        was_loading = true;
                                        if i % 6 == 0 {
                                            info!(
                                                "Report still loading... ({} min elapsed)",
                                                (i * 10) / 60
                                            );
                                        }
                                    } else {
                                        std::thread::sleep(Duration::from_secs(3));
                                        if let Ok(res_confirm) = tab.evaluate(check_loader_js, true)
                                        {
                                            if let Some(val_confirm) = res_confirm.value {
                                                if !val_confirm.as_bool().unwrap_or(false) {
                                                    if was_loading {
                                                        info!("Loader successfully disappeared.");
                                                    } else {
                                                        info!("Loader was never detected, assuming fast load or immediate result.");
                                                    }
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After Loader Disappears",
                        );
                        *step_num += 1;

                        // STEP 6: Export
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
                        javascript::evaluate_automation_step(
                            tab,
                            step6_export_js,
                            "Step 6 (Export)",
                        )?;
                        std::thread::sleep(Duration::from_secs(1));
                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After clicking Export",
                        );
                        *step_num += 1;

                        let step6_xlsx_js = r#"
                                        (async function() {
                                            let doc = document.querySelector('iframe').contentWindow.document;
                                            let mainDoc = document;
                                            let logs = [];
                                            let xlsxOption = null;

                                            // Try iframe first
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

                                            // Try main doc
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

                                            if (!xlsxOption) return JSON.stringify({ status: "ERROR", msg: "XLSX option not found.", logs });
                                            xlsxOption.click();
                                            logs.push("Clicked XLSX option");
                                            return JSON.stringify({ status: "SUCCESS", logs });
                                        })();
                                    "#;

                        info!("Clicking XLSX...");
                        javascript::evaluate_automation_step(tab, step6_xlsx_js, "Step 6 (XLSX)")?;
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
