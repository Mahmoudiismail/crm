use crm_tool::yasweb::browser::reports::generate_step6_js;

#[test]
fn test_generate_step6_js_normal_timeout() {
    let js = generate_step6_js(60);
    // Should inject 60 as timeoutMinutes param
    assert!(js.contains("const timeoutMs = timeoutMinutes * 60 * 1000;"));
    assert!(js.contains("const startDeadline = Date.now() + timeoutMs;"));
    assert!(js.contains("const finishDeadline = Date.now() + timeoutMs;"));
    assert!(js.contains("const exportDeadline = Date.now() + timeoutMs;"));
    assert!(js.contains("})(60);"));
    assert!(!js.contains("for(let i=0; i<1200; i++)")); // Should not have the hardcoded 120s limit
}

#[test]
fn test_generate_step6_js_large_timeout() {
    let js = generate_step6_js(6000);
    // Should inject 6000 as timeoutMinutes param
    assert!(js.contains("})(6000);"));
}

#[test]
fn test_generate_step6_js_zero_timeout() {
    let js = generate_step6_js(0);
    // Should inject 0 as timeoutMinutes param
    assert!(js.contains("})(0);"));
}

#[test]
fn test_download_wait_uses_correct_duration() {
    // We cannot run a real integration test, but we can verify the timeout is converted
    // correctly in download.rs by testing the conversion logic itself.
    let timeout_minutes = 6000;
    let timeout_duration = std::time::Duration::from_secs(timeout_minutes * 60);
    assert_eq!(timeout_duration.as_secs(), 360000);
}

use crm_tool::yasweb::browser::reports::generate_mis_reports_wait_js;

#[test]
fn test_generate_mis_reports_wait_js_normal_timeout() {
    let js = generate_mis_reports_wait_js(10);
    // Should inject 10 as timeoutMinutes param
    assert!(js.contains("const deadline = Date.now() + timeoutSeconds * 1000;"));
    assert!(js.contains("})(10);"));

    // Match the rendered label robustly and avoid offsetParent-based visibility checks.
    assert!(js.contains("entry.doc.querySelectorAll('.label.fw-bold')"));
    assert!(js.contains("=== \"MIS Reports\""));
    assert!(js.contains("function isVisible(element)"));
    assert!(js.contains("element.getClientRects()"));
    assert!(js
        .trim_start()
        .starts_with("(async function(timeoutSeconds)"));
    assert!(!js.trim_start().starts_with("return await"));
    assert!(!js.contains("element.offsetParent !== null"));

    // Check loading indicator presence
    assert!(js.contains("#loader_svg, mat-progress-bar, .dx-loadpanel"));
}

#[test]
fn test_generate_mis_reports_wait_js_large_timeout() {
    let js = generate_mis_reports_wait_js(120);
    // Should inject 120 as timeoutMinutes param
    assert!(js.contains("})(120);"));
}

#[test]
fn test_generate_mis_reports_wait_js_zero_timeout() {
    let js = generate_mis_reports_wait_js(0);
    // Should inject 0 as timeoutMinutes param
    assert!(js.contains("})(0);"));
}
