# Yasweb Runtime Errors — P0/P1/P2 Execution Plan

## Objective
Resolve the confirmed Yasweb runtime failures in date-filter validation, login input, iframe readiness/navigation, parallel browser isolation, and diagnostic capture without weakening report validation or disabling parallel execution.

## Scope and implementation sequence

### P0 — Correctness blockers
1. OwlDateTime date filters: inspect configured selectors and Angular/OwlDateTime DOM contract; replace plain value mutation with a component-accepted UI interaction; verify both date fields become valid before continuing; add focused tests.
2. Login username entry: inspect selectors, waits, CDP typing, and failure handling; wait for a visible/enabled/stable input; focus, clear, type, and verify value; retry only transient failures within a bounded deadline; preserve original cause; add regression tests.

### P1 — Runtime stability
3. Iframe readiness: distinguish transient polling from terminal failure; use a bounded deadline and structured diagnostics; avoid error-level logs for expected polling; emit a warning/error only at recovery/final deadline.
4. Refresh/navigation lifecycle: remove the iframe-readiness recovery refresh entirely; wait through the remaining configured timeout without discarding the SPA/iframe load. Keep route recovery separate and only before the MIS SPA route is reached.
5. Parallel browser isolation: trace browser/profile/tab ownership and shared state; preserve configured concurrency (default 6); ensure each report owns its Chrome profile/session and avoid holding browser tab locks across conflicting operations; add behavioral tests without lowering concurrency.

### P2 — Diagnostics
6. HTML/JS diagnostics: keep capture TRACE-gated and best-effort; publish unique files atomically, redact URL query strings/fragments in logs, preserve original browser errors if diagnostic writes fail, and test path uniqueness.

## Required repository workflow
- Keep all changes on this dedicated branch and target main with one PR.
- Update relevant md/ documentation in the same change.
- Run cargo fmt --all -- --check, focused Yasweb tests, cargo test -p yasweb, cargo clippy -p yasweb --all-targets -- -D warnings, and relevant workspace/CI checks available.
- Review the complete diff against this plan and the original request.
- Do not claim production browser success without a live Yasweb run.
- Do not merge the PR automatically; report CI and live-run limitations clearly.

## Acceptance criteria
- Date filters are accepted by the Angular form before report execution.
- Username entry is verified before login continues; transient CDP failures are bounded and diagnosable.
- Iframe waits do not trigger premature refreshes or log expected polling as terminal errors.
- Concurrent reports remain supported at the configured limit with isolated browser state.
- Diagnostics preserve the original failure and do not create misleading secondary failures.
- Tests and documentation cover each implemented behavior.