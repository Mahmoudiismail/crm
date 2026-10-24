# Plan: Yasweb MIS Wait and CRM Open Sohail CC

## Goals
1. Diagnose and fix the Yasweb `Wait for MIS Reports` automation returning `NO_RESULT` almost immediately instead of waiting for its asynchronous browser-side readiness check.
2. Wait for actual report-selector readiness in the Angular SPA, including the expected MIS route and accessible report iframe, with meaningful timeout diagnostics.
3. Add `miismail@fakeeh.care` as the final CC recipient on the CRM Open Sohail ReplyAll email without removing or reordering existing CC recipients.
4. Add focused regression coverage and update operational documentation.

## Constraints
- Minimal V1 changes; no new dependencies or frameworks.
- Preserve current report type selection, report search, filters, and export flow.
- Preserve all existing ReplyAll recipients and CC order; append the new address only if it is not already present.
- Keep the work on the existing PR #431 branch; do not create another branch or open another PR.
- Do not claim live browser or Windows Outlook verification unless actually performed.

## Implementation
1. Inspect `javascript::evaluate_automation_step`, the generated MIS wait JavaScript, and `headless_chrome::Tab::evaluate` semantics. Correct the async evaluation/result contract at the narrowest shared point that does not break other automation steps.
2. Ensure the MIS readiness script distinguishes route reached, Angular/iframe still loading, usable report type controls, visible search input, and a genuine timeout. Return structured JSON strings for success and timeout.
3. Append `miismail@fakeeh.care` to the ReplyAll message's CC list, preserving the existing CC string and avoiding a duplicate.
4. Add focused tests for the evaluator contract/readiness script and CC append/duplicate behavior where testable without a live browser or Outlook instance.
5. Update relevant documentation under `md/`.

## Verification
- Review the complete diff against this plan and `AGENTS.md`.
- Run `cargo fmt --all -- --check`, focused Yasweb and CRM Open Sohail tests, then the available CI-equivalent Clippy/test commands.
- Live Yasweb and Outlook/PowerShell behavior remain unverified unless run in their Windows/browser environment.


## Transfer Note
The implementation was transferred directly to `fix/runner-queued-status-run-all` (the head branch of PR #431) to comply with the one-PR/no-new-branch constraint. No additional pull request was opened. Automated checks and live Yasweb/Outlook verification must be confirmed separately; do not infer success from the transfer alone.
