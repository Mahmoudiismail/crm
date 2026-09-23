# Yasweb SPA Race Condition Fix

## Issue
In the `yasweb` binary (`crm_tool::yasweb::browser::reports`), there was a critical race condition when interacting with the Angular Single Page Application (SPA). After successfully clicking the "MIS module" button, the script immediately started polling the DOM for the "MIS Reports" button.

Because the application is an Angular SPA, there is a rendering delay (and a loading overlay like `#loader_svg` or `.loading-screen-wrapper`) before the new elements become available and fully interactive. The rapid polling (Rust-side `for` loop with `tab.find_element_by_xpath`) would fail to see the button or time out prematurely before the SPA had time to update the UI and remove the loading spinner.

## Fix
We replaced the Rust-side element polling loop with an injected JavaScript execution sequence using `javascript::evaluate_automation_step`.

The new approach, `generate_mis_reports_wait_js`:
1. Executes entirely in the browser context via `tab.evaluate(...)`.
2. First, loops and waits for any known SPA loading indicators (`#loader_svg`, `.loading-screen-wrapper`, `mat-progress-bar`, `.dx-loadpanel`) to disappear completely by checking both existence and actual `getComputedStyle` visibility (`display`, `opacity`, `visibility`).
3. Only after the loader is gone, it uses `document.evaluate` to continuously poll for the "MIS Reports" button using its specific XPath.
4. Checks that the "MIS Reports" element is actually visible to the user (`getComputedStyle` check) before declaring success.
5. Emits structured JSON logs natively, and gracefully propagates timeouts as `anyhow::Result::Err`, maintaining our safety invariants of zero production panics (`unwrap`/`expect`).

This brings the intermediate waiting logic in line with our existing robust patterns for UI interaction (e.g., `generate_step6_js`).

## Tests Added
Unit tests added in `tests/yasweb/timeout.rs` to verify that `generate_mis_reports_wait_js` accurately renders the JavaScript wait function, honoring the timeout limit and retaining the precise wait criteria.
