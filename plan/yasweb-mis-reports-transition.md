# Yasweb MIS Reports Transition — Implementation Plan

## Evidence
- The 2026-10-17 18:13 runner log succeeds at login, opens the module menu, clicks MIS, navigates to `/home/mis-report/module/...`, and then fails after 30 seconds waiting for “MIS Reports”.
- The timeout diagnostics show the matching text as `display:block`, `visibility:visible`, `opacity:1`, but `getClientRects().length == 0`; it is not a rendered/interactive control.
- The accessible iframe points to `https://yasweb.fakeeh.care:8065/auto-login`. The wait script must distinguish merely-present text from a usable MIS report selector.
- Prior HTML snapshots show an Angular SPA and the clicked MIS module, confirming asynchronous route/render transitions.

## Proposed implementation
1. Update only `generate_mis_reports_wait_js` in `src/yasweb/browser/reports.rs`.
2. During the existing 30-second deadline, inspect main and accessible iframe documents repeatedly, without treating text-only/zero-rectangle labels as ready.
3. Define readiness from the expected report route plus an actual usable MIS Reports control/content in the active document. Keep all selector checks defensive for missing/detached frames and log each document URL, candidate visibility/rect metrics, route state, and loader state on timeout.
4. Retain structured JSON results through the existing automation evaluator and keep the 30-second timeout.
5. Extend focused tests in `tests/yasweb/timeout.rs` to guard against false success from zero-rectangle candidates and to require route/content-aware diagnostics.
6. Update `md/yasweb-spa-race-condition.md` and `md/OPERATIONS.md`; commit this plan under `plan/`.

## Scope boundaries
- This is one PR only, against `main`; do not open another until this PR is closed.
- No changes to report filters, report-name search, XLSX export, Runner date forwarding, credentials, or concurrency.
- Smallest correct V1 fix; no new dependencies or new framework.

## Verification
- Review the complete diff against this plan.
- Run formatting and focused Yasweb timeout tests plus CI if available.
- Do not claim the production browser transition is fixed until a live Yasweb run proves the next phase and report list are usable.
