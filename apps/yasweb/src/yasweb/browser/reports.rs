use anyhow::{Context, Result};
use headless_chrome::protocol::cdp::Page;
use headless_chrome::Tab;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

use crate::yasweb::browser::{debug, javascript};
use crate::yasweb::config::DiscoveredReportFilters;

/// Navigates to an MIS report, applies the requested filters, and exports the report as XLSX.
///
/// Returns the filter labels discovered in the report form.
///
/// # Examples
///
/// ```no_run
/// use std::collections::HashMap;
/// use std::sync::Arc;
/// use headless_chrome::Tab;
/// use yasweb_app::yasweb::browser::reports::navigate_and_run_report;
///
/// # fn example() -> anyhow::Result<()> {
/// # let tab: Arc<Tab> = unimplemented!();
/// let mut step_num = 0;
/// let filters = HashMap::new();
///
/// let discovered = navigate_and_run_report(
///     &tab,
///     "Sales Report",
///     "Operational",
///     &filters,
///     10,
///     &mut step_num,
/// )?;
/// # Ok(())
/// # }
/// ```
///
/// # Arguments
///
/// * `active_report_name` - The report to select and export.
/// * `active_report_type` - The report category to select.
/// * `active_filters` - Filter values to apply before generating the report.
/// * `step_num` - Counter used to number saved browser-state snapshots.
///
/// # Returns
///
/// The filter labels and dropdown options discovered in the report form.
pub fn navigate_and_run_report(
    tab: &Arc<Tab>,
    active_report_name: &str,
    active_report_type: &str,
    active_filters: &HashMap<String, String>,
    timeout_minutes: u64,
    step_num: &mut u32,
) -> Result<DiscoveredReportFilters> {
    let report_started = Instant::now();
    info!(
        "YASWEB STAGE START: navigate_and_run_report report='{}' type='{}' filters_count={}",
        active_report_name,
        active_report_type,
        active_filters.len()
    );
    let mut discovered_filters = DiscoveredReportFilters::default();

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
                    debug::save_failure_diagnostics(tab, "menu_button_click");
                } else {
                    let check_js_eval = load_managed_browser_script(
                        "check_menu_state.js",
                        include_str!("scripts/check_menu_state.js"),
                    )?;
                    if let Ok(eval_result) = tab.evaluate(&check_js_eval, true) {
                        if let Some(val) = eval_result.value {
                            sidebar_toggled = val.as_bool().unwrap_or(false);
                        }
                    }

                    if !sidebar_toggled {
                        warn!("Menu '.menuModules' did not receive 'show-modules' class after waiting. MIS Reports might be inaccessible.");
                        let log_classes_js = load_managed_browser_script(
                            "read_menu_classes.js",
                            include_str!("scripts/read_menu_classes.js"),
                        )?;
                        if let Ok(eval_result) = tab.evaluate(&log_classes_js, true) {
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
                    if tab
                        .wait_for_element_with_custom_timeout(mis_selector, Duration::from_secs(10))
                        .is_ok()
                    {
                        mis_found = true;
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
                debug::save_failure_diagnostics(tab, "mis_module_wait_timeout");
                return Err(anyhow::anyhow!("MIS module wait timeout"));
            }

            match tab.wait_for_element(mis_selector) {
                Ok(mis_module) => {
                    info!("Clicking on MIS module...");
                    if let Err(e) = mis_module.click() {
                        error!("Failed to click MIS module: {:?}", e);
                        debug::save_failure_diagnostics(tab, "click_mis_module");
                        return Err(e).context("Yasweb stage failed: click MIS module");
                    } else {
                        info!("Clicked MIS successfully. Waiting for MIS Reports button...");
                        tracing::debug!("Waiting for MIS Reports button...");
                        info!("MIS module click returned; checking the SPA transition to MIS Reports.");

                        let mis_wait_started = Instant::now();
                        info!("YASWEB STAGE START: wait_for_mis_reports timeout_seconds=30");
                        let wait_js = generate_mis_reports_wait_js(30)?;
                        if let Err(first_error) =
                            evaluate_report_frame_step(tab, &wait_js, "Wait for MIS Reports")
                        {
                            error!(
                                "YASWEB STAGE RETRY: wait_for_mis_reports initial attempt failed elapsed_ms={} error={:#}",
                                mis_wait_started.elapsed().as_millis(),
                                first_error
                            );

                            // A failed transition can leave the SPA on the previous report route.
                            // Retry the MIS navigation once only when the MIS route was not reached.
                            match retry_mis_navigation(tab) {
                                Ok(true) => {
                                    info!("MIS route was not reached; retried the MIS module click once.");
                                    if let Err(retry_error) = evaluate_report_frame_step(
                                        tab,
                                        &wait_js,
                                        "Wait for MIS Reports (retry)",
                                    ) {
                                        error!(
                                            "YASWEB STAGE FAILED: wait_for_mis_reports after retry elapsed_ms={} error={:#}",
                                            mis_wait_started.elapsed().as_millis(),
                                            retry_error
                                        );
                                        debug::save_failure_diagnostics(
                                            tab,
                                            "wait_for_mis_reports_retry",
                                        );
                                        return Err(retry_error).context("Yasweb stage failed: wait for MIS Reports after navigation retry");
                                    }
                                }
                                Ok(false) => {
                                    // The route was reached, so this was likely a slow embedded UI.
                                    // Retry the readiness wait without clicking navigation again.
                                    info!("MIS route already reached; retrying readiness wait once without another navigation click.");
                                    if let Err(retry_error) = evaluate_report_frame_step(
                                        tab,
                                        &wait_js,
                                        "Wait for MIS Reports (readiness retry)",
                                    ) {
                                        error!(
                                            "YASWEB STAGE FAILED: wait_for_mis_reports after readiness retry elapsed_ms={} error={:#}",
                                            mis_wait_started.elapsed().as_millis(),
                                            retry_error
                                        );
                                        debug::save_failure_diagnostics(
                                            tab,
                                            "wait_for_mis_reports_readiness_retry",
                                        );
                                        return Err(retry_error).context("Yasweb stage failed: wait for MIS Reports after readiness retry");
                                    }
                                }
                                Err(recovery_error) => {
                                    error!(
                                        "Failed to recover MIS navigation after initial readiness failure: {:#}",
                                        recovery_error
                                    );
                                    debug::save_failure_diagnostics(tab, "mis_navigation_recovery");
                                    return Err(first_error)
                                        .context("Yasweb stage failed: wait for MIS Reports; navigation recovery also failed");
                                }
                            }
                        }
                        info!(
                            "YASWEB STAGE END: wait_for_mis_reports status=SUCCESS elapsed_ms={}",
                            mis_wait_started.elapsed().as_millis()
                        );
                        info!("MIS Reports button successfully verified.");
                        println!("MIS Reports button successfully verified.");
                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After clicking MIS module",
                        );
                        *step_num += 1;

                        info!("Searching for auto-login iframe...");
                        let mut iframe_node_id = None;

                        if let Ok(iframe_node) = tab
                            .wait_for_element_with_custom_timeout("iframe", Duration::from_secs(10))
                        {
                            iframe_node_id = Some(iframe_node.node_id);
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
                        let step1_js = generate_step1_js(active_report_type)?;

                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "Before selecting Report Type",
                        );
                        *step_num += 1;
                        info!("Selecting Report Type: {}", active_report_type);
                        evaluate_report_frame_step(tab, &step1_js, "Step 1 (Report Type)")?;
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
                        let step2_js = load_managed_browser_script("select_report.js", include_str!("scripts/select_report.js"))?
                            .replace("__REPORT_TYPE_JSON__", &serde_json::to_string(&active_report_type)?)
                            .replace("__REPORT_NAME_JSON__", &serde_json::to_string(&active_report_name)?)
                            .replace("__TIMEOUT_MINUTES__", &timeout_minutes.to_string());

                        info!("Searching & Selecting Report: {}", active_report_name);
                        evaluate_report_frame_step(tab, &step2_js, "Step 2 (Select Report)")?;
                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After Selecting Report from List",
                        );
                        *step_num += 1;

                        // STEP 3: Wait for binding
                        tracing::debug!("Executing JavaScript to wait for binding...");
                        let step3_js = load_managed_browser_script("wait_report_binding.js", include_str!("scripts/wait_report_binding.js"))?
                            .replace("__REPORT_NAME_JSON__", &serde_json::to_string(&active_report_name)?)
                            .replace("__TIMEOUT_MINUTES__", &timeout_minutes.to_string());

                        info!("Waiting for Report Binding...");
                        evaluate_report_frame_step(tab, &step3_js, "Step 3 (Binding)")?;
                        debug::save_html_state(
                            tab,
                            active_report_name,
                            *step_num,
                            "After Report Bound",
                        );
                        *step_num += 1;

                        // STEP 3.5 & 4: Wait for Loader to disappear, then fill filters
                        tracing::debug!("Executing JavaScript to apply filters...");
                        let step4_fill_js = load_managed_browser_script("fill_filters.js", include_str!("scripts/fill_filters.js"))?
                            .replace("__FILTERS_JSON__", &filters_json)
                            .replace("__TIMEOUT_MINUTES__", &timeout_minutes.to_string())
                            .replace("__TRACE_ENABLED__", if tracing::enabled!(tracing::Level::TRACE) { "true" } else { "false" });

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
                        let step5_search_js = load_managed_browser_script("click_search.js", include_str!("scripts/click_search.js"))?
                            .replace("__TIMEOUT_MINUTES__", &timeout_minutes.to_string());

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
                        let step6_export_js = generate_step6_js(timeout_minutes)?;

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
                        let step7_xlsx_js = load_managed_browser_script("select_xlsx.js", include_str!("scripts/select_xlsx.js"))?
                            .replace("__TIMEOUT_MINUTES__", &timeout_minutes.to_string());

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

fn load_managed_browser_script(logical_name: &str, source: &str) -> Result<String> {
    let path = crm_tool::script_manager::ScriptManager::new()
        .get_or_create_script("Yasweb", logical_name, source)
        .with_context(|| format!("Failed to install managed Yasweb script '{}'", logical_name))?;
    std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read managed Yasweb script '{}'", path.display()))
}

/// Waits for the Angular SPA to finish routing into the MIS report module.
///
/// Clicking a module only starts client-side navigation; the URL can remain on
/// the dashboard briefly. Do not fail or click the module again during that
/// transition window.
fn wait_for_mis_reports_route(tab: &Arc<Tab>, timeout: Duration) -> Result<()> {
    let started = Instant::now();
    let deadline = started + timeout;
    while Instant::now() < deadline {
        let current_url = tab.get_url();
        if is_mis_reports_route(&current_url) {
            info!(
                "YASWEB MIS SPA route reached: url='{}' elapsed_ms={}",
                current_url,
                started.elapsed().as_millis()
            );
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let last_url = tab.get_url();
    anyhow::bail!(
        "Timed out waiting {}ms for MIS SPA route; last_url='{}'",
        timeout.as_millis(),
        last_url
    )
}

fn is_mis_reports_route(url: &str) -> bool {
    url.contains("/home/mis-report/module/")
}

/// Runs a report automation step in the embedded report frame's own CDP execution context.
/// The report UI is cross-origin from the parent page, so parent-page JavaScript cannot read its DOM.
fn evaluate_report_frame_step(
    tab: &Arc<Tab>,
    js_code: &str,
    step_name: &str,
) -> Result<serde_json::Value> {
    if step_name.starts_with("Wait for MIS Reports") {
        wait_for_mis_reports_route(tab, Duration::from_secs(30))
            .with_context(|| format!("{}: MIS SPA route did not become active", step_name))?;
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

fn generate_step1_js(report_type: &str) -> Result<String> {
    Ok(load_managed_browser_script("select_report_type.js", include_str!("scripts/select_report_type.js"))?
        .replace("__REPORT_TYPE_JSON__", &serde_json::to_string(report_type)?))
}

#[cfg(test)]
mod report_type_js_tests {
    use super::generate_step1_js;

    #[test]
    fn report_type_selection_avoids_full_dom_serialization_and_broad_xpath() {
        let js = generate_step1_js("Standard Report").unwrap();
        assert!(js.contains("querySelectorAll('mat-radio-button')"));
        assert!(js.contains("querySelectorAll('label')"));
        assert!(!js.contains(".innerHTML"));
        assert!(!js.contains("doc.evaluate"));
        assert!(!js.contains("normalize-space"));
        assert!(js.contains("\"Standard Report\""));
    }
}

pub fn generate_step6_js(timeout_minutes: u64) -> Result<String> {
    Ok(load_managed_browser_script("wait_export.js", include_str!("scripts/wait_export.js"))?
        .replace("__TIMEOUT_MINUTES__", &timeout_minutes.to_string()))
}

/// Recovers a missed MIS navigation click without disturbing a route that already changed.
/// Returns true when the MIS module was clicked, false when the MIS route was already reached.
fn retry_mis_navigation(tab: &Arc<Tab>) -> Result<bool> {
    let script = load_managed_browser_script("retry_mis_navigation.js", include_str!("scripts/retry_mis_navigation.js"))?;
    let result = tab.evaluate(&script, true).context("Failed to evaluate MIS navigation recovery")?;
    let value = result.value.and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| anyhow::anyhow!("MIS navigation recovery returned no result"))?;
    let parsed: serde_json::Value = serde_json::from_str(&value)
        .context("MIS navigation recovery returned invalid JSON")?;
    if parsed.get("status").and_then(serde_json::Value::as_str) != Some("SUCCESS") {
        let message = parsed.get("msg").and_then(serde_json::Value::as_str)
            .unwrap_or("MIS navigation recovery failed");
        anyhow::bail!("{message}");
    }
    Ok(parsed.get("clicked").and_then(serde_json::Value::as_bool).unwrap_or(false))
}

pub fn generate_mis_reports_wait_js(timeout_seconds: u64) -> Result<String> {
    Ok(load_managed_browser_script("wait_mis_reports.js", include_str!("scripts/wait_mis_reports.js"))?
        .replace("__TIMEOUT_SECONDS__", &timeout_seconds.to_string()))
}

#[cfg(test)]
mod mis_route_tests {
    use super::is_mis_reports_route;

    #[test]
    fn recognizes_the_mis_reports_spa_route() {
        assert!(is_mis_reports_route(
            "https://yasweb.fakeeh.care/home/mis-report/module/H5F296ADEE1A17F02E06364BB010ACC5C"
        ));
    }

    #[test]
    fn does_not_treat_the_dashboard_as_the_mis_reports_route() {
        assert!(!is_mis_reports_route(
            "https://yasweb.fakeeh.care/home/dashboard"
        ));
    }
}

#[cfg(test)]
mod step6_wait_tests {
    use super::generate_step6_js;

    #[test]
    fn waits_for_export_button_to_become_enabled() {
        let js = generate_step6_js(5).unwrap();
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
        let js = generate_mis_reports_wait_js(30).unwrap();
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
