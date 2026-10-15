# Security & Reliability Audit — Current Status

## Scope and status

This document originated as a historical audit. Some findings in the original version no longer match the current code and must not be treated as confirmed current vulnerabilities without revalidation. This update corrects findings directly contradicted by the current source; it does not claim that every historical concern has been exhaustively re-audited.

## Findings checked against current source

### Runner HTTP request bounds — previous unbounded-read finding is stale

Current `src/runner/gui/mod.rs` defines `MAX_HEADER_BYTES = 64 * 1024` and `MAX_BODY_BYTES = 2 * 1024 * 1024`, and applies a 10-second timeout while reading requests. The previous statement that the server resizes the request buffer without bounds is not accurate for the current implementation.

**Status:** The original unbounded-read finding is superseded by these bounds. This is not a claim that every HTTP protocol edge case (for example, all transfer-encoding behavior) has been exhaustively tested.

### CSV row-width validation — previous `.flexible(true)` finding is stale

The current `src/tasker/csv_task/reader.rs` uses the shared `crate::utils::build_csv_reader_from_reader` helper rather than the old `csv_task.rs` implementation described in the historical finding. The repository’s current CSV-reader documentation also describes strict parsing as the default and requires explicit opt-in for flexible row widths.

**Status:** The original location and evidence are stale; do not report `.flexible(true)` in `src/tasker/csv_task.rs` as a current confirmed defect.

### Runner GUI network exposure — qualify the earlier unauthenticated-route claim

Current `src/runner/config/validation.rs` restricts GUI bind addresses to loopback values (`127.0.0.1`, `localhost`, `::1`, and `[::1]`). Routes should still be treated as privileged local controls, but the historical claim of direct remote-network access is not supported without evidence of a non-loopback binding path that bypasses validation.

**Status:** Keep local access control as a hardening consideration; do not describe remote RCE as confirmed from the route handlers alone.

## Current V1 follow-up

### Runner config load errors in task handlers

In `src/runner/gui/handlers.rs`, task creation, task update, and the new-task page previously used `RunnerConfig::load(...).unwrap_or_default()`, while other pages propagated load errors. The edit page also loaded the same configuration a second time after already loading it successfully. The V1 follow-up changes these task handlers to propagate load errors consistently and reuse the already-loaded edit-page configuration.

**Expected behavior:** If configuration loading fails, the handler returns an error rather than continuing with a fabricated default configuration. This change does not alter the config schema or public API.

## Historical findings requiring fresh evidence before being classified as open defects

The following claims appeared in the original audit but have not been confirmed as current defects by this update. Reinspect their current call paths and trust boundaries before implementing a fix:

- Synchronous filesystem work in `src/runner/engine/logging.rs` and its actual impact on Tokio scheduling.
- Absolute path handling in `src/runner/engine/validation.rs`; allowing an explicitly configured absolute path is not automatically a path traversal vulnerability.
- PowerShell string interpolation in `src/tasker/department_split.rs`; trace each interpolated value to establish whether untrusted input can reach script source.
- The 60-second headless-browser wait on failure; establish whether it remains present and whether the delay is required by the current debugging contract.
- Secret handling beyond the custom `Debug` redaction already present; identify a concrete leak path before proposing a larger secret-type migration.
- HTTP protocol edge cases not covered by the current size and timeout limits.

These are **revalidation candidates**, not current confirmed findings or an instruction to perform broad refactors.

## Deferred resource risk

### Unbounded CRM download queue

The download queue in `src/crm/fetcher.rs` remains a resource-risk candidate. Do not replace it with a bounded channel and awaited sends without addressing consumer failure: the processor can stop on an unrecoverable download error, which could leave producers blocked while the queue is no longer being drained.

**Status:** Deferred. Any future change must include safe failure/drain behavior and a focused behavioral test for backpressure without deadlock or lost work.

## Verification policy

- The source observations above are based on targeted inspection, not a claim that the entire repository was re-audited.
- PR #410 passed Format and Auto-fix, Clippy Lints, and Test Suite on PR-head commit `c44a58bbc098c5bc4fbed75f21400b4cfbe7726d` in [workflow run 38648046753](https://github.com/Mahmoudiismail/crm/actions/runs/38648046753).
- A separate CI run on merge commit `b493602a6cc6e20f91ee3070ea1224217f783975` was not found when the previous status was recorded. The commit status query currently returns no reported statuses, which is not equivalent to a failing run.
- Do not claim local Cargo checks or post-merge CI unless those checks have actually been run and their results inspected.

## V1 constraints

Prefer the smallest correct production fix. Avoid architecture rewrites, unnecessary abstractions, public API expansion, or test infrastructure that is disproportionate to the behavior being changed. Keep broad refactors and nonessential coverage improvements out of V1.
