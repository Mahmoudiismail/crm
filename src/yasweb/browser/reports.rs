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
                                                                        if (opt.textContent.toLowerCase().includes(value.toLowerCase())) {{                                                                            opt.click();
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
                                                        const input = findFilterControl(lbl);
                                                        if (input) {{
                                                            if (input.tagName === 'INPUT') {{
                                                                let v = value;
                                                                if (key.toLowerCase().includes('date') && v.includes('-')) {{
                                                                    const parts = v.split(' ')[0].split('-');
                                                                    if (parts.length === 3) {{
                                                                        v = parts[0].padStart(2, '0') + "-" + parts[1].padStart(2, '0') + "-" + parts[2] + (v.includes(' ') ? ' ' + v.split(' ').slice(1).join(' ') : '');
                                                                    }}
                                                                }}
                                                                await simulateTyping(input, v);
                                                                logs.push("Typed into INPUT for " + key);
                                                                filterFilled = true;
                                                                break;
                                                            }} else if (input.tagName === 'MAT-SELECT' || input.getAttribute('role') === 'combobox') {{
                                                                input.click();
                                                                await sleep(300);
                                                                const overlay = doc.querySelector('.cdk-overlay-container') || doc;
                                                                const options = Array.from(overlay.querySelectorAll('mat-option, [role="option"]'));
                                                                const selectedOption = options.find(option => normalize(option.innerText || option.textContent).includes(normalize(value)));
                                                                if (selectedOption) {{
                                                                    selectedOption.click();
                                                                    logs.push("Selected dropdown option for " + key);
                                                                    filterFilled = true;
                                                                }} else {{
                                                                    const backdrop = doc.querySelector('.cdk-overlay-backdrop');
                                                                    if (backdrop) backdrop.click();
                                                                    logs.push("Dropdown option not found for " + key + ": " + value);
                                                                }}
                                                                await sleep(300);
                                                                break;
                                                            }} else if (input.tagName === 'SELECT') {{
                                                                const option = Array.from(input.options).find(item =>
                                                                    normalize(item.textContent) === normalize(value) || item.value === value
                                                                );
                                                                if (option) {{
                                                                    input.value = option.value;
                                                                    input.dispatchEvent(new Event('input', {{ bubbles: true }}));
                                                                    input.dispatchEvent(new Event('change', {{ bubbles: true }}));
                                                                    logs.push("Selected native SELECT option for " + key);
                                                                    filterFilled = true;
                                                                }} else {{
                                                                    logs.push("Native SELECT option not found for " + key + ": " + value);
                                                                }}
                                                                break;
                                                            }}
                                                        }}
                                                    }}
                                                }}
                                                if (!filterFilled) {{
                                                    logs.push("Failed to fill filter: " + key);
                                                    failedFilters.push(key);
                                                }}
                                            }}
                                            if (failedFilters.length > 0) {{
                                                return JSON.stringify({{ status: "ERROR", msg: "One or more requested filters could not be applied: " + failedFilters.join(", "), failed_filters: failedFilters, discovered_filters: discoveredFilters, discovered_filter_options: discoveredFilterOptions, logs }});
                                            }}
                                            return JSON.stringify({{ status: "SUCCESS", discovered_filters: discoveredFilters, discovered_filter_options: discoveredFilterOptions, logs }});
                                        }})({});
                                        "#,
                            filters_json
                        );

                        info!("Applying Filters...");
                        let step4_res = evaluate_report_frame_step(
                            tab,
                            &step4_fill_js,
                            "Step 4 (Fill Filters)",
                        )?;
                        if let Some(arr) = step4_res
                            .get("discovered_filters")
                            .and_then(|v| v.as_array())
                        {
                            for filter in arr {
                                if let Some(name) = filter.as_str() {
                                    discovered_filters.names.push(name.to_string());
                                }
                            }
                        }
                        if let Some(options) = step4_res
                            .get("discovered_filter_options")
                            .and_then(|value| value.as_object())
                        {
                            for (filter, values) in options {
                                let values = values
                                    .as_array()
                                    .into_iter()
                                    .flatten()
                                    .filter_map(serde_json::Value::as_str)
                                    .map(str::to_owned)
                                    .collect::<Vec<_>>();
                                if !values.is_empty() {
                                    discovered_filters.options.insert(filter.clone(), values);
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

                        // STEP 5: Search Click
                        tracing::debug!("Executing JavaScript to click search...");
                        let step5_search_js = r#"
                                        (async function(timeoutMinutes) {
                                            function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }
                                            let logs = [];
                                            let doc = document;

                                            logs.push("Waiting for Search button to appear...");
                                            let clickedSearch = false;
                                            const deadlineSearch = Date.now() + timeoutMinutes * 60 * 1000;
                                            while (Date.now() <= deadlineSearch) {
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
                                                await sleep(200);
                                            }

                                            if (!clickedSearch) {
                                                return JSON.stringify({ status: "ERROR", msg: "Search button not found.", logs });
                                            }
                                            return JSON.stringify({ status: "SUCCESS", logs });
                                        })( {} );
                                    "#;
                        let step5_search_js =
                            step5_search_js.replace("{}", &timeout_minutes.to_string());

                        info!("Clicking Search...");
                        evaluate_report_frame_step(tab, &step5_search_js, "Step 5 (Search Click)")?;

                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After Clicking Search",
                        );
                        *step_num += 1;

                        // STEP 6: Wait for Loader and Click Export
                        tracing::debug!("Executing JavaScript to wait for export generation...");
                        let step6_export_js = generate_step6_js(timeout_minutes);

                        info!("Waiting for generation & clicking Export...");
                        evaluate_report_frame_step(
                            tab,
                            &step6_export_js,
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
                                        (async function(timeoutMinutes) {
                                            function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }
                                            let doc = document;
                                            let mainDoc = document;
                                            let logs = [];
                                            let xlsxOption = null;
                                            const deadlineXlsx = Date.now() + timeoutMinutes * 60 * 1000;
                                            while (Date.now() <= deadlineXlsx) {
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
                                                await sleep(200);
                                            }

                                            if (!xlsxOption) return JSON.stringify({ status: "ERROR", msg: "XLSX option not found.", logs });
                                            xlsxOption.click();
                                            logs.push("Clicked XLSX option");
                                            return JSON.stringify({ status: "SUCCESS", logs });
                                        })( {} );
                                    "#;
                        let step7_xlsx_js =
                            step7_xlsx_js.replace("{}", &timeout_minutes.to_string());

                        info!("Clicking XLSX...");
                        evaluate_report_frame_step(tab, &step7_xlsx_js, "Step 7 (XLSX)")?;
                        info!("JS Automation Sequence Completed Successfully!");

                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After clicking XLSX",
                        );
                    }
                }
                Err(e) => {
                    error!("Failed to find MIS module: {:?}", e);
                    debug::save_failure_diagnostics(tab, "find_mis_module");
                    return Err(e).context("Yasweb stage failed: find MIS module");
                }
            }
        }
        Err(e) => {
            error!("Failed to find #menuPinnedBtn: {:?}", e);
            debug::save_failure_diagnostics(tab, "find_menu_button");
            return Err(e).context("Yasweb stage failed: find menu button");
        }
    }

    info!("YASWEB STAGE END: navigate_and_run_report status=SUCCESS elapsed_ms={} discovered_filters={}", report_started.elapsed().as_millis(), discovered_filters.len());
    Ok(discovered_filters)
}

/// Runs a report automation step in the embedded report frame's own CDP execution context.
/// The report UI is cross-origin from the parent page, so parent-page JavaScript cannot read its DOM.
fn evaluate_report_frame_step(
    tab: &Arc<Tab>,
    js_code: &str,
    step_name: &str,
) -> Result<serde_json::Value> {
    if step_name.starts_with("Wait for MIS Reports")
        && !tab.get_url().contains("/home/mis-report/module/")
    {
        anyhow::bail!(
            "MIS navigation route was not reached before report-frame readiness check: {}",
            tab.get_url()
        );
    }

    let started = Instant::now();
    let iframe_src = tab
        .evaluate("document.querySelector('iframe')?.src || ''", true)
        .ok()
        .and_then(|result| result.value)
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut last_frame_url = String::new();
    let execution_context_id = loop {
        let tree = tab
            .call_method(Page::GetFrameTree(None))
            .context("Failed to inspect Chrome frame tree for Yasweb report iframe")?;
        let root_origin = tree.frame_tree.frame.security_origin.clone();
        let mut frames = Vec::new();
        collect_child_frames(&tree.frame_tree, &mut frames);
        let frame = frames
            .iter()
            .find(|frame| !iframe_src.is_empty() && frame.url == iframe_src)
            .or_else(|| {
                frames
                    .iter()
                    .find(|frame| !iframe_src.is_empty() && iframe_src.starts_with(&frame.url))
            })
            .or_else(|| {
                frames
                    .iter()
                    .find(|frame| frame.security_origin != root_origin && !frame.url.is_empty())
            });
        if let Some(frame) = frame {
            last_frame_url = frame.url.clone();
            match tab.call_method(Page::CreateIsolatedWorld {
                frame_id: frame.id.clone(),
                world_name: Some("crm-yasweb-report".to_string()),
                grant_univeral_access: Some(false),
            }) {
                Ok(world) => break world.execution_context_id,
                Err(error) if Instant::now() < deadline => tracing::debug!(
                    "Report frame context not ready yet (url={}): {}",
                    frame.url,
                    error
                ),
                Err(error) => {
                    return Err(error).context("Failed to create report-frame execution context")
                }
            }
        }
        if Instant::now() >= deadline {
            anyhow::bail!("Could not locate embedded Yasweb report frame within 10 seconds (iframe_src='{}', last_frame_url='{}')", iframe_src, last_frame_url);
        }
        std::thread::sleep(Duration::from_millis(200));
    };
    info!(
        "YASWEB report-frame context ready: stage='{}' frame_url='{}' elapsed_ms={}",
        step_name,
        last_frame_url,
        started.elapsed().as_millis()
    );
    javascript::evaluate_automation_step_in_context(tab, js_code, step_name, execution_context_id)
}

fn collect_child_frames<'a>(tree: &'a Page::FrameTree, frames: &mut Vec<&'a Page::Frame>) {
    if let Some(children) = &tree.child_frames {
        for child in children {
            frames.push(&child.frame);
            collect_child_frames(child, frames);
        }
    }
}

fn generate_step1_js(report_type: &str) -> String {
    // Avoid repeatedly serializing the full iframe markup or running broad XPath queries.
    // The report page can contain a large Angular DOM, so inspect only the relevant controls.
    let template = r#"
        (async function(reportType) {
            function sleep(ms) { return new Promise(resolve => setTimeout(resolve, ms)); }
            const logs = [];
            const doc = document;

            const normalize = value => (value || "").replace(/\s+/g, " ").trim().toLowerCase();
            const target = normalize(reportType);

            // Wait for the iframe document to exist without repeatedly materializing innerHTML.
            for (let i = 0; i < 40; i++) {
                if (doc.body && doc.body.childElementCount > 0) break;
                await sleep(250);
            }

            for (let i = 0; i < 40; i++) {
                const radios = doc.querySelectorAll('mat-radio-button');
                let candidate = null;
                for (const radio of radios) {
                    if (normalize(radio.textContent).includes(target)) {
                        candidate = radio;
                        break;
                    }
                }
                if (candidate) {
                    const input = candidate.querySelector('input[type="radio"]');
                    if (input) {
                        input.click();
                        input.dispatchEvent(new Event('change', { bubbles: true }));
                    } else {
                        candidate.click();
                    }
                    logs.push("Selected report type using mat-radio-button");
                    return JSON.stringify({ status: "SUCCESS", logs });
                }

                for (const label of doc.querySelectorAll('label')) {
                    if (normalize(label.textContent).includes(target)) {
                        label.click();
                        logs.push("Selected report type using label fallback");
                        return JSON.stringify({ status: "SUCCESS", logs });
                    }
                }

                await sleep(250);
            }

            return JSON.stringify({ status: "ERROR", msg: "Report type not found: " + reportType, logs });
        })(__REPORT_TYPE_JSON__);
    "#;

    template.replace(
        "__REPORT_TYPE_JSON__",
        &serde_json::to_string(report_type).unwrap_or_else(|_| "\"\"".to_string()),
    )
}

#[cfg(test)]
mod report_type_js_tests {
    use super::generate_step1_js;

    #[test]
    fn report_type_selection_avoids_full_dom_serialization_and_broad_xpath() {
        let js = generate_step1_js("Standard Report");
        assert!(js.contains("querySelectorAll('mat-radio-button')"));
        assert!(js.contains("querySelectorAll('label')"));
        assert!(!js.contains(".innerHTML"));
        assert!(!js.contains("doc.evaluate"));
        assert!(!js.contains("normalize-space"));
        assert!(js.contains("\"Standard Report\""));
    }
}

pub fn generate_step6_js(timeout_minutes: u64) -> String {
    format!(
        r#"
        (async function(timeoutMinutes) {{
            function sleep(ms) {{ return new Promise(resolve => setTimeout(resolve, ms)); }}
            const logs = [];
            const timeoutMs = timeoutMinutes * 60 * 1000;
            const pollMs = 1000;

            function getReportDocument() {{
                return document;
            }}

            function isVisible(element) {{
                if (!element) return false;
                const view = element.ownerDocument && element.ownerDocument.defaultView;
                if (!view) return false;
                const style = view.getComputedStyle(element);
                return style.display !== 'none' &&
                    style.visibility !== 'hidden' &&
                    style.opacity !== '0' &&
                    element.getClientRects().length > 0;
            }}

            function findVisibleLoader() {{
                const selector = '#loader_svg, .loading-screen-wrapper, mat-progress-bar, .dx-loadpanel';
                const reportDoc = getReportDocument();
                const docs = [document];
                if (reportDoc && reportDoc !== document) docs.push(reportDoc);
                for (const doc of docs) {{
                    for (const element of doc.querySelectorAll(selector)) {{
                        if (isVisible(element)) return element;
                    }}
                }}
                return null;
            }}

            function findExportButton() {{
                const reportDoc = getReportDocument();
                if (!reportDoc) return null;

                const candidates = [];
                const labelled = reportDoc.querySelector('div[aria-label="Export"], button[aria-label="Export"]');
                if (labelled) candidates.push(labelled);

                for (const element of reportDoc.querySelectorAll('.dx-button-text')) {{
                    if (element.textContent.trim().toLowerCase() === 'export') {{
                        const button = element.closest('.dx-button, div[role="button"], button');
                        if (button && !candidates.includes(button)) candidates.push(button);
                    }}
                }}

                for (const element of reportDoc.querySelectorAll('button, div[role="button"], .dx-button')) {{
                    if (element.textContent.trim().toLowerCase() === 'export' && !candidates.includes(element)) {{
                        candidates.push(element);
                    }}
                }}
                return candidates.find(isVisible) || null;
            }}

            function isExportEnabled(button) {{
                if (!button || !isVisible(button)) return false;
                const disabledClasses = ['disabled', 'disable', 'dx-state-disabled', 'mat-button-disabled'];
                if (disabledClasses.some(className => button.classList.contains(className))) return false;
                if (button.disabled === true ||
                    button.getAttribute('disabled') !== null ||
                    button.getAttribute('aria-disabled') === 'true') return false;
                const view = button.ownerDocument.defaultView;
                if (view.getComputedStyle(button).pointerEvents === 'none') return false;
                return true;
            }}

            // A missing loader is not proof that the report is ready. Wait for the
            // Export control itself to become enabled; the app toggles its disabled
            // state asynchronously after the report data has finished binding.
            logs.push("Waiting for the Export button to become enabled...");
            const deadline = Date.now() + timeoutMs;
            let lastState = "";
            while (Date.now() <= deadline) {{
                const loader = findVisibleLoader();
                const button = findExportButton();
                const enabled = isExportEnabled(button);
                if (button) {{
                    const state = JSON.stringify({{
                        classes: button.className && button.className.toString(),
                        ariaDisabled: button.getAttribute('aria-disabled'),
                        disabled: button.getAttribute('disabled') !== null,
                        loaderVisible: !!loader
                    }});
                    if (state !== lastState) {{
                        logs.push("Export state: " + state);
                        lastState = state;
                    }}
                }} else if (lastState !== "missing") {{
                    logs.push("Export button is not present yet.");
                    lastState = "missing";
                }}

                if (button && enabled && !loader) {{
                    // Re-check on the next poll so a transient enabled state during
                    // report refresh does not trigger an early export.
                    await sleep(500);
                    const latestButton = findExportButton();
                    if (latestButton && isExportEnabled(latestButton) && !findVisibleLoader()) {{
                        latestButton.click();
                        logs.push("Clicked enabled Export button.");
                        return JSON.stringify({{ status: "SUCCESS", logs }});
                    }}
                }}
                await sleep(pollMs);
            }}

            const button = findExportButton();
            return JSON.stringify({{
                status: "ERROR",
                msg: button
                    ? "Export button did not become enabled before timeout."
                    : "Export button not found before timeout.",
                logs
            }});
        }})({});
        "#,
        timeout_minutes
    )
}

/// Recovers a missed MIS navigation click without disturbing a route that already changed.
/// Returns true when the MIS module was clicked, false when the MIS route was already reached.
fn retry_mis_navigation(tab: &Arc<Tab>) -> Result<bool> {
    let script = r##"
        (async function() {
            const routePattern = /\/home\/mis-report\/module\//i;
            if (routePattern.test(location.href)) return JSON.stringify({ status: "SUCCESS", clicked: false });

            const menu = document.querySelector("#menuPinnedBtn:not(.d-none), #pinButton:not(.d-none)");
            const modules = document.querySelector(".menuModules");
            if (!modules || !modules.classList.contains("show-modules")) {
                if (!menu) return JSON.stringify({ status: "ERROR", msg: "MIS navigation recovery could not find the menu button." });
                menu.click();
                const menuDeadline = Date.now() + 5000;
                while (Date.now() < menuDeadline) {
                    const current = document.querySelector(".menuModules");
                    if (current && current.classList.contains("show-modules")) break;
                    await new Promise(resolve => setTimeout(resolve, 100));
                }
            }

            const deadline = Date.now() + 5000;
            while (Date.now() < deadline) {
                const module = document.querySelector(".misManagement");
                if (module) {
                    const style = getComputedStyle(module);
                    const visible = style.display !== "none" && style.visibility !== "hidden" &&
                        Array.from(module.getClientRects()).some(rect => rect.width > 0 && rect.height > 0);
                    if (visible) {
                        module.click();
                        return JSON.stringify({ status: "SUCCESS", clicked: true });
                    }
                }
                await new Promise(resolve => setTimeout(resolve, 100));
            }
            return JSON.stringify({ status: "ERROR", msg: "MIS navigation recovery could not find a visible MIS module." });
        })()
    "##;

    let result = tab
        .evaluate(script, true)
        .context("Failed to evaluate MIS navigation recovery")?;
    let value = result
        .value
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| anyhow::anyhow!("MIS navigation recovery returned no result"))?;
    let parsed: serde_json::Value =
        serde_json::from_str(&value).context("MIS navigation recovery returned invalid JSON")?;
    if parsed.get("status").and_then(serde_json::Value::as_str) != Some("SUCCESS") {
        let message = parsed
            .get("msg")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("MIS navigation recovery failed");
        anyhow::bail!("{message}");
    }
    Ok(parsed
        .get("clicked")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false))
}

pub fn generate_mis_reports_wait_js(timeout_seconds: u64) -> String {
    format!(
        r##"
        return await (async function(timeoutSeconds) {{
            function sleep(ms) {{ return new Promise(resolve => setTimeout(resolve, ms)); }}
            const logs = [];
            const deadline = Date.now() + timeoutSeconds * 1000;
            const typeNames = ["Standard Report", "Report Manager"];

            function normalize(value) {{ return (value || "").replace(/\s+/g, " ").trim().toLowerCase(); }}
            function isVisible(element) {{
                if (!element) return false;
                const view = element.ownerDocument && element.ownerDocument.defaultView;
                if (!view) return false;
                for (let current = element; current && current.nodeType === 1; current = current.parentElement) {{
                    const style = view.getComputedStyle(current);
                    if (style.display === "none" || style.visibility === "hidden" || style.visibility === "collapse" || Number(style.opacity) === 0) return false;
                }}
                return Array.from(element.getClientRects()).some(rect => rect.width > 0 && rect.height > 0);
            }}
            function findTypeControl(typeName) {{
                const target = normalize(typeName);
                return Array.from(document.querySelectorAll('mat-radio-button, [role="radio"], input[type="radio"], label, button'))
                    .find(element => isVisible(element) && normalize(element.textContent || element.getAttribute("aria-label") || element.value).includes(target)) || null;
            }}
            function inspect() {{
                const reportTypes = typeNames.map(name => ({{ name, visible: !!findTypeControl(name) }}));
                const searchInput = document.querySelector('input[formcontrolname="searchInput"], input[placeholder="Search"], input[aria-label*="Search" i]');
                const loaderSelector = "#loader_svg, .loading-screen-wrapper, mat-progress-bar, .dx-loadpanel";
                const visibleLoaders = Array.from(document.querySelectorAll(loaderSelector)).filter(isVisible).map(element => ({{
                    selector: loaderSelector,
                    tag: element.tagName,
                    className: typeof element.className === "string" ? element.className : ""
                }}));
                return {{ frameUrl: location.href, documentReadyState: document.readyState, reportTypes,
                    searchVisible: isVisible(searchInput), visibleLoaders,
                    reportUiReady: reportTypes.every(item => item.visible) && isVisible(searchInput) && visibleLoaders.length === 0 }};
            }}
            logs.push("Waiting inside the report-frame execution context for visible report-type controls and Search...");
            let lastState = "";
            while (Date.now() <= deadline) {{
                const state = inspect();
                const serialized = JSON.stringify(state);
                if (serialized !== lastState) {{ logs.push("Report-frame readiness state: " + serialized); lastState = serialized; }}
                if (state.reportUiReady) return JSON.stringify({{ status: "SUCCESS", logs }});
                await sleep(250);
            }}
            return JSON.stringify({{ status: "ERROR", msg: "Report frame did not expose visible Standard Report, Report Manager, and Search controls before timeout",
                logs: logs.concat(["Report-frame readiness timeout diagnostics: " + JSON.stringify(inspect())]) }});
        }})({});
        "##,
        timeout_seconds
    )
}

#[cfg(test)]
mod step6_wait_tests {
    use super::generate_step6_js;

    #[test]
    fn waits_for_export_button_to_become_enabled() {
        let js = generate_step6_js(5);
        assert!(js.contains("Waiting for the Export button to become enabled"));
        assert!(js.contains("'dx-state-disabled'"));
        assert!(js.contains("getAttribute('aria-disabled') === 'true'"));
        assert!(js.contains("button.disabled === true"));
        assert!(js.contains("Export button did not become enabled before timeout."));
        assert!(js.contains("})(5);"));
    }
}

#[cfg(test)]
mod mis_reports_wait_tests {
    use super::generate_mis_reports_wait_js;

    #[test]
    fn wait_script_returns_the_async_result_expected_by_automation_evaluator() {
        let js = generate_mis_reports_wait_js(30);
        assert!(js
            .trim_start()
            .starts_with("return await (async function(timeoutSeconds)"));
        assert!(js.contains("})(30);"));
    }

    #[test]
    fn wait_script_checks_accessible_iframes_and_reports_diagnostics() {
        let js = generate_mis_reports_wait_js(30);
        assert!(!js.contains("contentWindow.document"));
        assert!(js.contains("Waiting inside the report-frame execution context"));
        assert!(js.contains("Report-frame readiness timeout diagnostics"));
        assert!(js.contains("visibleLoaders"));

        assert!(js.contains("reportUiReady"));
        assert!(js.contains("reportTypes"));
        assert!(js.contains("status: \"SUCCESS\""));
        assert!(js.contains("status: \"ERROR\""));
    }
}
