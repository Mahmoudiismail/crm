# Plan: Fix Yasweb MIS Reports Wait

## Goal
Fix the failure immediately after clicking the MIS module, where the automation reports that the JavaScript step returned no result string. Keep the V1 change narrow and preserve the existing report-selection/export flow.

## Evidence
- Latest Yasweb logs show login, menu opening, and clicking the MIS module succeed.
- `Wait for MIS Reports` fails after approximately 17 ms with `returned no result string`, far earlier than its 30-second timeout.
- The generated script starts with `return await (async function ...)`, unlike the other automation scripts that return an expression value to `Tab.evaluate(..., true)`.
- The captured main document does not contain the exact `MIS Reports` label, while an auto-login iframe is present. The wait should inspect the main document and accessible same-origin iframe documents and report useful diagnostics if the label is still missing.

## Implementation
1. Change the generated JavaScript to evaluate an async IIFE expression directly, so Chrome's promise-await evaluation receives its returned JSON string.
2. Search the main document and accessible iframe documents for the visible `MIS Reports` label and loader state; do not fail merely because the label is rendered in an accessible iframe.
3. Preserve timeout behavior and return explicit JSON `SUCCESS` or `ERROR`; include inspected frame URLs and matching-label diagnostics in timeout errors.
4. Add focused unit tests for the generated script's key contract if the current module structure allows testing without a live browser.

## Verification
- Run formatting, focused tests, full tests, and Clippy through CI.
- Inspect the diff and CI results.
- A successful CI run does not replace a live Yasweb browser run; verify that the report flow proceeds to report selection and export using the built executable.

## Scope constraints
No broad refactor, new framework, unrelated report-flow changes, or changes to filters/export behavior.
