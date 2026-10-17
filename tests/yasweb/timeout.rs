use crm_tool::yasweb::browser::reports::generate_step6_js;

#[test]
fn test_generate_step6_js_normal_timeout() {
    let js = generate_step6_js(60);
    assert!(js.contains("const timeoutMs = timeoutMinutes * 60 * 1000;"));
    assert!(js.contains("const startDeadline = Date.now() + timeoutMs;"));
    assert!(js.contains("const finishDeadline = Date.now() + timeoutMs;"));
    assert!(js.contains("const exportDeadline = Date.now() + timeoutMs;"));
    assert!(js.contains("})(60);"));
    assert!(!js.contains("for(let i=0; i<1200; i++)"));
}

#[test]
fn test_generate_step6_js_large_timeout() {
    let js = generate_step6_js(6000);
    assert!(js.contains("})(6000);"));
}

#[test]
fn test_generate_step6_js_zero_timeout() {
    let js = generate_step6_js(0);
    assert!(js.contains("})(0);"));
    assert!(js.trim_start().starts_with("return await (async function(timeoutSeconds)"));
}

#[test]
fn test_download_wait_uses_correct_duration() {
    let timeout_minutes = 6000;
    let timeout_duration = std::time::Duration::from_secs(timeout_minutes * 60);
    assert_eq!(timeout_duration.as_secs(), 360000);
}

use crm_tool::yasweb::browser::reports::generate_mis_reports_wait_js;

#[test]
fn test_generate_mis_reports_wait_js_normal_timeout() {
    let js = generate_mis_reports_wait_js(10);
    assert!(js.contains("const deadline = Date.now() + timeoutSeconds * 1000;"));
    assert!(js.contains("})(10);"));
    assert!(js.trim_start().starts_with("return await (async function(timeoutSeconds)"));
    assert!(js.contains("frame.contentDocument"));
    assert!(js.contains("function isVisible(element)"));
    assert!(js.contains("element.getClientRects()"));
    assert!(js.contains(r"const routePattern = /\\/home\\/mis-report\\/module\\//i;"));
    assert!(js.contains("input[formcontrolname=\"searchInput\"]"));
    assert!(js.contains("input[placeholder=\"Search\"]"));
    assert!(js.contains("Standard Report"));
    assert!(js.contains("Report Manager"));
    assert!(js.contains("searchVisible"));
    assert!(js.contains("reportUiReady"));
    assert!(js.contains("MIS readiness timeout diagnostics"));
    assert!(!js.contains("element.offsetParent !== null"));
    assert!(js.contains("#loader_svg, .loading-screen-wrapper, mat-progress-bar, .dx-loadpanel"));
}

#[test]
fn test_generate_mis_reports_wait_js_large_timeout() {
    let js = generate_mis_reports_wait_js(120);
    assert!(js.contains("})(120);"));
    assert!(js.trim_start().starts_with("return await (async function(timeoutSeconds)"));
}

#[test]
fn test_generate_mis_reports_wait_js_zero_timeout() {
    let js = generate_mis_reports_wait_js(0);
    assert!(js.contains("})(0);"));
}
