# Full Repository SOLID & DRY Audit — V1

## Status
The audit remediation was merged into `main` through [PR #410](https://github.com/Mahmoudiismail/crm/pull/410) on 2026-10-14. Merge commit: [`b493602a6cc6e20f91ee3070ea1224217f783975`](https://github.com/Mahmoudiismail/crm/commit/b493602a6cc6e20f91ee3070ea1224217f783975). This document records the verified result of that work; it does not authorize additional code changes or merges.

## Confirmed findings and remediation
- **Security/privacy — CRM response payload logging (implemented and merged in PR #410):** `src/crm/fetcher.rs` logged the full incomplete-reservation PATCH body and full CRM API response bodies at DEBUG; arbitrary response bodies were also embedded in several HTTP errors and malformed JSON contexts. Responses can contain signed download URLs, report data, and ticket IDs. The change now logs response metadata/byte count, omits PATCH body contents (logs only ticket count), and sanitizes HTTP/parse errors. Focused unit tests verify that response bodies are omitted from errors while the `failed to generate signed url` marker needed by recursive date-range splitting is preserved.
- **P1 — Users Base64 processing failure was swallowed (fixed and merged in PR #410):** the users-report future logged `process_base64_payload` errors and still returned `Ok((key, value))`. It now propagates the error with report context. A focused async regression test supplies malformed Base64 and asserts the failure reaches the caller.
- **P1 — Download processor could outlive a failed report fetch (fixed and merged in PR #410):** `try_collect().await?` returned early before awaiting the spawned download processor. The fetch result is now retained, the processor is always joined, and only then is the fetch error propagated. A focused async test verifies that the processor has completed before a fetch error returns.
- **DRY/documentation:** Consolidated the fetcher logging rule and corrected concurrency/failure documentation in `md/FETCHER.md`.
- Historical findings were checked against current code before being considered: current Tasker task errors are propagated with `bail!`, and the CRM download processor reports exhausted download retries as errors. These are not re-reported as open defects.
- **Unbounded download queue — assessed, no change yet:** the queue can accumulate tasks, but naively switching to bounded `mpsc` plus awaited sends is unsafe with the current `try_for_each_concurrent` processor: the processor exits on its first unrecoverable download error, which could stop draining the queue and leave producers blocked on `send().await`. Keep the current channel until the processor is changed to drain queued tasks while recording a failure and a behavioral backpressure test can verify no deadlock/lost task. This remains a resource-risk candidate, not a confirmed production defect.

## Additional findings now remediated
- YASWEB false-success handling, signed download URL disclosure, and stale CSV-reader documentation were addressed in PR #410 and are merged into `main`. Their focused regression tests and documentation updates are included in the merged changes.

## Follow-up fixes from continued scan
- **YASWEB false-success handling (implemented):** `evaluate_automation_step` now rejects malformed JSON, missing/non-string statuses, and every status except explicit `SUCCESS` or `ERROR`. Added focused tests for malformed JSON, missing status, unknown status, success, and error. Updated `md/YASWEB_DEV.md` to state this contract.
- **Signed URL disclosure in downloader (implemented):** `download_csv` no longer logs the signed URL, includes it in transport errors, or includes arbitrary HTTP response bodies in failures. Transport errors use `reqwest::Error::without_url()`; HTTP failures report only report key and status.
- **CSV documentation drift (implemented):** `md/ARCHITECTURE.md` now documents the strict default reader and requires callers to opt in explicitly when flexible row widths are intended.
- These fixes were verified by the successful pull-request CI run recorded below. No local Cargo run is claimed in this plan.

## Current verification status
- PR #410 was merged into `main` on 2026-10-14: https://github.com/Mahmoudiismail/crm/pull/410. Merge commit: https://github.com/Mahmoudiismail/crm/commit/b493602a6cc6e20f91ee3070ea1224217f783975.
- The final PR-head CI run [38648046753](https://github.com/Mahmoudiismail/crm/actions/runs/38648046753) passed on head commit `c44a58bbc098c5bc4fbed75f21400b4cfbe7726d`: **Format and Auto-fix**, **Clippy Lints**, and **Test Suite** all succeeded.
- The earlier run [38643230918](https://github.com/Mahmoudiismail/crm/actions/runs/38643230918) failed one test because the expected sanitized HTTP status text was incomplete; commit `c44a58b` corrected the expectation, and the final PR-head run passed afterward.
- Verification qualification: the successful run was on the PR head before merge. At the time this status was updated, no independent CI run on merge commit `b493602a6cc6e20f91ee3070ea1224217f783975` was found. Therefore this plan does not claim a separate post-merge CI run.

## Repository and constraints
- Repository: `Mahmoudiismail/crm`
- Target branch: `main`
- Current project: one Cargo package with multiple binaries (not a true Cargo workspace).
- Scope: repository-wide audit of production Rust code, tests, CI/configuration and impacted Markdown documentation.
- Primary rule: smallest correct V1 change. No enterprise architecture, unnecessary abstraction, public API expansion, broad rewrite, or file splitting solely to reduce line count.
- Follow repository `AGENTS.md`: incremental commits per distinct fix; tests proving behavior; documentation updates for operational changes; final diff review; never use `git reset`; do not claim checks not actually run.

## Objectives
1. Find and prioritize correctness, security, data integrity, concurrency, resource lifecycle and error-propagation defects.
2. Apply SOLID only where there is a concrete maintenance or correctness benefit.
3. Remove genuinely duplicated logic under DRY while preserving domain-specific behavior and avoiding premature generalization.
4. Improve performance/resource usage only where evidence supports it and behavior remains compatible.
5. Preserve all existing CLI/configuration formats, manifest behavior, release semantics and observable workflows unless a verified defect requires a documented correction.
6. Keep V1 changes focused; classify optional cleanup/large restructures as V2 recommendations instead of expanding execution scope.

## Audit phases

### Phase 1 — Establish current baseline
- Read current `AGENTS.md`, documentation policy, architecture docs, prior audit plans, and recent merged PRs.
- Enumerate all `src/**/*.rs`, tests, Cargo.toml/Cargo.lock, and GitHub workflows.
- Avoid re-reporting historical findings without verifying current code.
- Run baseline checks where available before changes: `cargo fmt --all -- --check`, targeted tests, `cargo test --all-targets --all-features`, and `cargo clippy --all-targets --all-features -- -D warnings`. If toolchain/runtime is unavailable, record the limitation and rely on CI only when observed.

### Phase 2 — Production-correctness and security audit
Review every production module for:
- Errors logged and swallowed, incorrect success/exit codes, partial operations reported as successful.
- Panic paths (`unwrap`, `expect`, `panic!`, unchecked assumptions) outside tests and whether they are truly reachable.
- File writes, temp-file cleanup, atomic persistence, config read-modify-write races and lock scope.
- Process spawn, child/process-tree termination, timeout, cancellation, shutdown, output limits and handle cleanup.
- Async functions that perform blocking I/O or hold guards/locks across slow operations.
- HTTP status handling, bounded request/response bodies, retry/idempotency semantics, authentication token refresh and secret leakage in logs.
- CSV malformed-row behavior, row/header invariants, business-rule filters, path handling and accidental data exposure.
- Windows COM/PowerShell invocation, argument separation, exception cleanup and failure propagation.
- Workflow permissions, release safety and accidental operational data/secret exposure.
Only label an issue confirmed after tracing current code and relevant call paths.

### Phase 3 — SOLID and DRY review
- Single Responsibility: identify functions/modules combining unrelated decision, I/O, rendering, parsing or orchestration logic where that coupling is concretely harmful.
- Open/Closed and Dependency Inversion: assess current seams and injected functions already used in tests; add a seam only when it materially improves correctness/testability without introducing a framework.
- Interface Segregation / Liskov: check existing abstractions and behavior contracts; do not add trait hierarchies for hypothetical future use.
- DRY: compare duplicated business logic and parsing/normalization rules across binaries. Reuse shared functions only if semantics are genuinely identical; preserve domain-specific exceptions.
- Review large modules (`src/runner/config/schedule.rs`, `src/crm/fetcher.rs`, `src/yasweb/browser/reports.rs`, `src/runner/gui/templates.rs`, `src/utils.rs`, and large Tasker modules) for high coupling and duplication. File size by itself is not a defect.
- Check allocations, repeated serialization/parsing, unnecessary cloning/collection, and stream/file handling without speculative optimization.

### Phase 4 — Prioritization and implementation boundaries
Classify findings:
- P0: data loss/corruption, security exposure, serious unsafe runtime behavior, false-success behavior in a production-critical flow.
- P1: reproducible functional defects, race conditions, resource leaks, important reliability/correctness problems.
- P2: clear duplication or maintainability issues with low behavior risk.
- V2/defer: broad redesigns, speculative abstractions, cosmetic refactors, large module splits without demonstrated benefit.
For each finding record: evidence (path/function and relevant code), scenario, impact, severity, smallest safe fix, focused regression test, docs impacted, and dependencies/order.

### Phase 5 — Approved implementation, only after START
- Create a dedicated branch from the reviewed current `main` head.
- Address P0/P1 first; take each distinct fix in a small, reviewable commit.
- Refactor only when supported by an accepted finding. Avoid behavior changes bundled with style-only refactors.
- Add regression tests for behavior fixed; prefer runtime/behavioral tests for concurrency, process lifecycle, persistence and injection boundaries.
- Update relevant `md/` documentation in the same changes. Update this plan's execution status as work proceeds.
- Run targeted tests after each fix, then the full required suite. Re-run affected verification after any later change.
- Manually review the complete final diff against the original request and this plan before proposing a PR.
- Do not merge unless separately requested.

## Verification matrix
- Format: `cargo fmt --all -- --check`
- Tests: `cargo test --all-targets --all-features`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Build: `cargo build --release` for the configured Windows target through CI where local Windows-specific dependencies prevent native Linux validation.
- Security/dependency checks: inspect configured workflow and lockfile; run `cargo audit` / `cargo deny` only if configured/available and report exact outcomes.
- Domain focused tests: Runner schedule/pipeline/concurrency; CRM authentication/download/error propagation; YASWEB selectors/navigation/waits; Tasker CSV/report/email/COM script contracts; updater script/download/version safety.
- CI status and actual run URLs/results recorded in final report. No unverifiable claims.

## Initial observations (hypotheses to verify; not yet findings)
- Repository contains 124 Rust files; the largest production source files include schedule.rs, fetcher.rs, reports.rs and gui/templates.rs. Review coupling/duplication rather than splitting by size.
- Existing shared utilities and modularized Runner GUI already exist; avoid reintroducing duplicate infrastructure.
- Prior audit documents include potential Tasker error swallowing and CRM download failure propagation concerns. Re-check current implementation and commit history before classifying or fixing.
- Recent PR #409 touched YASWEB report type selection and waiting logic; verify the merged implementation and its CI outcome before proposing related changes.
- Recent hardening work changed CI/release/data handling; verify current workflows rather than relying solely on previous plan descriptions.

## Acceptance criteria
1. The audit covers repository-wide production code and relevant tests/docs, not just selected files.
2. Findings are evidence-based, deduplicated against current fixes, risk-ranked, and mapped to clear minimal actions.
3. SOLID/DRY changes remove real coupling/duplication without unnecessary frameworks or API churn.
4. Every behavior fix has a focused regression test when applicable and documentation updates where behavior changes.
5. Required format/test/Clippy/build checks have actual recorded results or explicitly documented environment limitations.
6. Full final diff and scope are reviewed; no changes are merged without separate authorization.
