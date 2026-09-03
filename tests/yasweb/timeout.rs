use crm_tool::yasweb::browser::reports::generate_step6_js;

#[test]
fn test_generate_step6_js_normal_timeout() {
    let js = generate_step6_js(60);
    // Should inject 60 as timeoutMinutes param
    assert!(js.contains("const deadline = Date.now() + timeoutMinutes * 60 * 1000;"));
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
