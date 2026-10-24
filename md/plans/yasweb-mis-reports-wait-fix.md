# Yasweb MIS Reports Wait Failure — Implementation Plan

## Evidence
- The 2026-10-17 Runner task log reaches login success, opens the pinned menu, finds and clicks the MIS module, then times out waiting for the MIS Reports label.
- The captured HTML at failure contains the MIS Reports label with the expected `label fw-bold` classes and a `.third-level` submenu, but the outer `.menuModules` no longer has `show-modules`.
- `generate_mis_reports_wait_js` waits for the outer loader to disappear first and then requires the label to be visible via a single XPath candidate and `offsetParent`.
- The archive of earlier HTML states also confirms the page is an Angular SPA with changing menu/module rendering.
- Earlier Yasweb errors in the separate log include XLSX option misses and CDP “No session with given id” / closed connections, which are separate symptoms and may relate to concurrent browser sessions. The current task's immediate blocker is the MIS Reports wait.

## Scope
1. Fix only the MIS Reports transition/visibility wait in `src/yasweb/browser/reports.rs`.
2. Make waiting tolerant of Angular rendering and duplicate/hidden matching labels by scanning all matching candidates and testing computed visibility robustly.
3. Do not claim success merely because hidden markup contains the text. Require a visible, interactive candidate in the active menu state.
4. Preserve the existing 30-second transition timeout. Check every matching loader and avoid `offsetParent` as the sole visibility signal.
5. Add focused regression coverage to `tests/yasweb/timeout.rs` or module unit tests that asserts generated JavaScript uses candidate iteration and robust visibility checks, with no stale single-node/offsetParent dependency.
6. Update the relevant `md/` Yasweb documentation and this plan.

## Non-goals
- Do not change Runner CLI argument forwarding or PR #422.
- Do not change report search, report filters, export handling, timeout configuration, or concurrency behavior unless investigation proves it is the cause of the current wait failure.
- Do not change production authentication or credentials.

## Verification
- Review the complete patch and current Yasweb flow.
- Run targeted timeout tests and formatting if toolchain is available.
- Confirm CI status after the branch push; do not claim tests passed unless an actual check reports success.
- Review timeout diagnostics and ensure generated JavaScript explicitly returns the awaited async result.
