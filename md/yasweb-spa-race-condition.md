# Yasweb SPA Race Condition Fix

## Issue
In the `yasweb` binary (`crm_tool::yasweb::browser::reports`), there was a critical race condition when interacting with the Angular Single Page Application (SPA). After successfully clicking the "MIS module" button, the script immediately started polling the DOM for the "MIS Reports" button.

Because the application is an Angular SPA, there is a rendering delay (and a loading overlay like `#loader_svg` or `.loading-screen-wrapper`) before the new elements become available and fully interactive. The rapid polling (Rust-side `for` loop with `tab.find_element_by_xpath`) would fail to see the button or time out prematurely before the SPA had time to update the UI and remove the loading spinner.

Furthermore, we discovered in browser testing that the `.loading-screen-wrapper` check and `element.offsetParent !== null` reliance was leading to infinite hangs during generation steps in certain SPA states.

## Fix
We replaced the Rust-side element polling loop with an injected JavaScript execution sequence using `javascript::evaluate_automation_step`.

The updated approach, `generate_mis_reports_wait_js`:
1. Executes entirely in the browser context via `tab.evaluate(...)`.
2. Loops and waits for known SPA loading indicators (`#loader_svg`, `mat-progress-bar`, `.dx-loadpanel`) to disappear completely by checking their existence and actual `getComputedStyle` visibility (`display`, `opacity`, `visibility`). We strictly **removed** the `.loading-screen-wrapper` check as it was confirmed to cause permanent hangups in the live environment.
3. Only after the loader is gone, it uses `document.evaluate` to continuously poll for the "MIS Reports" button using its specific XPath.
4. Checks that the "MIS Reports" element is actually visible to the user via a strict `getComputedStyle` check (`display !== 'none'` and `visibility !== 'hidden'`), safely **removing** `element.offsetParent !== null` to avoid unexpected hangups.
5. Emits structured JSON logs natively, and gracefully propagates timeouts as `anyhow::Result::Err`, maintaining our safety invariants of zero production panics (`unwrap`/`expect`).
6. Enforces a 30-second hardcoded timeout (`timeout_seconds: 30`) specifically during the MIS Reports step transition, removing generic timeout injection limits (`timeout_minutes`).

This brings the intermediate waiting logic perfectly in line with our existing robust patterns for UI interaction (e.g., `generate_step6_js`), while precisely tuning out false-positive hangups for `loading-screen-wrapper` and `offsetParent` layout calculations.

## Tests Added
Unit tests added in `tests/yasweb/timeout.rs` verify that `generate_mis_reports_wait_js` accurately renders the JavaScript wait function, injecting `timeout_seconds` scaling natively inside JavaScript, and strictly enforcing the adjusted loader and visibility selectors.
