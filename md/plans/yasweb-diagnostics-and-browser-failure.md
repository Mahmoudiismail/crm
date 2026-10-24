# Yasweb Failure Diagnostics and Browser Lifecycle Fix

## Goal
Make Yasweb failures diagnosable by recording the exact automation stage, elapsed time, browser/page state, and JavaScript diagnostics, while ensuring failed runs are not reported as successful. Preserve the existing report flow and keep the fix narrowly scoped.

## Confirmed findings
- `run_browser_tab` returns immediately when login/report automation fails. The browser handle then drops, so the visible browser closes when `keep_open` is false.
- `javascript::evaluate_automation_step` currently logs only a compact error for JS evaluation failures and discards the returned diagnostic logs from the error chain.
- The runner's Yasweb task closure logs a browser automation error but returns an empty filter list, which can make the outer task path continue as if execution had completed.
- PR #423 (MIS Reports visibility wait) is already merged into `main`; the attached log's short generic timeout message may come from a binary built before that merge. This needs a version/build check and must not be assumed to be the only failure.

## Implementation
1. Add structured stage timing/logging around browser startup, login, report navigation, download wait, and cleanup.
2. Improve JavaScript automation errors so the returned message preserves the original error and step logs. Record the step name and elapsed time.
3. On automation failure, save page diagnostics (main URL/title/readiness, frame metadata, and HTML snapshot) to the existing debug output location. Avoid recording passwords, cookies, authorization headers, or full query strings.
4. Respect `keep_open` on failure, but make the behavior explicit in logs. Do not silently force a long delay for normal successful runs.
5. Propagate task failure through the Yasweb task result path so an automation failure cannot be logged as successful completion or proceed as if filters were successfully discovered.
6. Add focused tests for error-message/log preservation, safe URL redaction, and result/error handling where practical.
7. Update `md/YASWEB_DEV.md` with how to enable/use diagnostics and where artifacts are saved.

## Verification
- Run `cargo fmt --check`.
- Run focused Yasweb tests, then `cargo test` and `cargo clippy --all-targets --all-features -- -D warnings` if the available environment supports them.
- Review the final diff for unrelated changes and sensitive-data leakage.
- Browser-dependent behavior must be reported as unverified unless tested against the actual Yasweb site with the built executable.

## Constraints
- Minimal V1 change; no new logging framework or abstraction layer.
- No password/cookie/session-token logging.
- Do not claim runtime success based on compilation or CI alone.
