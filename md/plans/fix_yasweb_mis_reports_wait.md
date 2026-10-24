# Plan: Fix Yasweb MIS Reports Wait and Preserve Diagnostics

## Goal
Fix the failure immediately after clicking the MIS module and ensure later Yasweb failures do not overwrite earlier diagnostic artifacts.

## Evidence
- Latest Yasweb logs show login, menu opening, and clicking the MIS module succeed.
- `Wait for MIS Reports` fails after approximately 17 ms with `returned no result string`, far earlier than its 30-second timeout.
- The generated script starts with `return await (async function ...)`, unlike the other automation scripts that return an expression value to `Tab.evaluate(..., true)`.
- Routine HTML snapshots currently use deterministic filenames based on step number, step name, and report name, so a later run can overwrite them.
- Failure diagnostics use second-resolution Unix timestamps, so same-step failures in the same second can collide.

## Implementation
1. Change the generated JavaScript to evaluate an async IIFE expression directly, so Chrome's promise-await evaluation receives its returned JSON string.
2. Search the main document and accessible iframe documents for the visible `MIS Reports` label and loader state; include useful diagnostics if the label is missing.
3. Make diagnostic artifacts collision-resistant so repeated runs retain prior evidence instead of overwriting files.
4. Keep the change focused; do not refactor report filters or export behavior.

## Verification
- Run formatting, focused tests, full tests, and Clippy through CI.
- Inspect the diff and CI results.
- A successful CI run does not replace a live Yasweb browser run; verify the report flow against the real environment.

## Scope constraints
No broad refactor, new framework, unrelated report-flow changes, or changes to filters/export behavior.
