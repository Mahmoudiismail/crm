# Yasweb MIS Reports Transition — Implementation Plan

## Evidence
- The 2026-10-17 18:13 runner log succeeds at login, opens the module menu, clicks MIS, navigates to `/home/mis-report/module/...`, and then fails after 30 seconds waiting for “MIS Reports”.
- The timeout diagnostics show the matching text as `display:block`, `visibility:visible`, `opacity:1`, but `getClientRects().length == 0`; it is not a rendered/interactive control.
- The accessible iframe points to `https://yasweb.fakeeh.care:8065/auto-login`. The wait script must distinguish merely-present text from a usable MIS report selector.
- Prior HTML snapshots show an Angular SPA and the clicked MIS module, confirming asynchronous route/render transitions.

## Proposed implementation
1. Update `generate_mis_reports_wait_js` in `src/yasweb/browser/reports.rs` to treat the main MIS route and embedded report UI as separate documents.
2. During the existing timeout, inspect the main document and accessible iframes, requiring the MIS route plus visible `Standard Report`, `Report Manager`, and `Search` controls in the report document. A route or text label alone must not pass readiness.
3. Keep the existing workflow order: select `active_report_type` (either `Standard Report` or `Report Manager`), then search for `active_report_name` in that selected type, select the matching result, and only then continue filters/export. Do not hard-code one report type for all reports.
4. Keep iframe access defensive, preserve structured JSON results through the existing automation evaluator, and emit route/report-document/type/search/loader diagnostics on timeout.
5. Add focused regression assertions in `src/yasweb/browser/reports.rs` and `tests/yasweb/timeout.rs` for both report types, iframe search readiness, and the evaluator return contract.

## Scope boundaries
- This is one PR only, against `main`; do not open another until this PR is closed.
- No changes to report filters, report-name search, XLSX export, Runner date forwarding, credentials, or concurrency.
- Smallest correct V1 fix; no new dependencies or new framework.

## Verification
- Review the complete diff against this plan.
- Run formatting and focused Yasweb timeout tests plus CI if available.
- Do not claim the production browser transition is fixed until a live Yasweb run proves the next phase and report list are usable.
