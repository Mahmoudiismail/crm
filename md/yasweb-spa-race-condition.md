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
3. Only after visible SPA loaders disappear, it scans all `.label.fw-bold` candidates and matches the normalized visible text `MIS Reports`, rather than relying on a single XPath result.
4. Checks the element and its ancestors for hidden styles and requires a non-empty rendered rectangle. It does not rely on `offsetParent`, which can be null for elements that are still rendered.
5. Emits structured JSON logs natively, and gracefully propagates timeouts as `anyhow::Result::Err`, maintaining our safety invariants of zero production panics (`unwrap`/`expect`).
6. Enforces a 30-second hardcoded timeout (`timeout_seconds: 30`) specifically during the MIS Reports step transition, removing generic timeout injection limits (`timeout_minutes`).

This brings the intermediate waiting logic perfectly in line with our existing robust patterns for UI interaction (e.g., `generate_step6_js`), while precisely tuning out false-positive hangups for `loading-screen-wrapper` and `offsetParent` layout calculations.

## Tests Added
Unit tests added in `tests/yasweb/timeout.rs` verify that `generate_mis_reports_wait_js` accurately renders the JavaScript wait function, injecting `timeout_seconds` scaling natively inside JavaScript, and strictly enforcing the adjusted loader and visibility selectors.


## Follow-up: MIS Reports visibility regression (2026-10-17)

The latest failure HTML contained the `MIS Reports` label, but the runtime still rejected it because the implementation retained an `offsetParent !== null` condition despite the earlier documentation stating it had been removed. The wait now checks all matching labels, verifies computed visibility through the ancestor chain, and confirms the element has a rendered rectangle. Loader detection also checks all matching loader elements instead of only the first match. On timeout, diagnostic visibility details are returned in the structured logs.


## Follow-up: MIS transition readiness correction (2026-10-17)

The latest production run reached the MIS module route, but the wait still timed out because the matching label reported `display:block`, `visibility:visible`, and `opacity:1` while having no client rectangles. A label's text and computed styles alone are not a reliable readiness signal. The wait now polls the main document and accessible iframes until a rendered, interactive `MIS Reports` control or visible report-list controls exist on the expected MIS module route. It retains the 30-second deadline and records inspected document URLs, route matches, candidate counts, usable-control state, report-control state, and visible loader documents on timeout.


## Follow-up: async evaluator return contract (2026-10-18)

The MIS readiness script uses a top-level `return await` so its result is the final structured JSON string, but Chrome's `Runtime.evaluate` expects an expression. The shared automation evaluator now wraps only scripts beginning with `return await` in an async function expression before evaluation with Promise awaiting enabled. Other existing IIFE expressions are passed through unchanged. Focused tests protect both paths. This prevents the evaluator from receiving an undefined result before the browser-side readiness loop completes.


## Follow-up: delayed SPA route was failing before the readiness wait (2026-10-31)

A later production run exposed an earlier race than the report-frame readiness
logic: `evaluate_report_frame_step` checked the parent tab URL once and returned
an error immediately when the SPA was still on the dashboard. The route changed
to `/home/mis-report/module/...` shortly afterward, so the 30-second browser
readiness script never ran. The recovery path could then click MIS a second time
while the first navigation was still completing.

The route guard now polls the parent tab URL every 100 ms for up to 30 seconds
before discovering the report iframe and creating its CDP execution context.
This keeps the existing 30-second readiness wait, and prevents a second MIS
click merely because Angular has not updated the URL yet. The existing recovery
click remains available only after the full route wait expires. The frame is
still discovered from the current frame tree after the route is reached, so a
replaced iframe is not tied to a stale frame ID.

Focused tests cover MIS-route recognition and ensure the integration path uses
a bounded route wait rather than the former fail-fast guard. A live YASWEB run
is still required to verify the deployed Angular transition and cross-origin
frame behavior.
