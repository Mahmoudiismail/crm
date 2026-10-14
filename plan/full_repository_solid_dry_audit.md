# Full Repository SOLID & DRY Audit — V1

## Status
Implementation authorized by the user's explicit **START**. Work is being performed on `audit/solid-dry-v1-oct-2026`; do not merge into `main` without separate authorization.

## Confirmed findings and remediation
- **Security/privacy — CRM response payload logging (implemented on this branch):** `src/crm/fetcher.rs` logged the full incomplete-reservation PATCH body and full CRM API response bodies at DEBUG; arbitrary response bodies were also embedded in several HTTP errors and malformed JSON contexts. Responses can contain signed download URLs, report data, and ticket IDs. The change now logs response metadata/byte count, omits PATCH body contents (logs only ticket count), and sanitizes HTTP/parse errors. Focused unit tests verify that response bodies are omitted from errors while the `failed to generate signed url` marker needed by recursive date-range splitting is preserved.
- **P1 — Users Base64 processing failure was swallowed (fixed on this branch):** the users-report future logged `process_base64_payload` errors and still returned `Ok((key, value))`. It now propagates the error with report context. A focused async regression test supplies malformed Base64 and asserts the failure reaches the caller.
- **P1 — Download processor could outlive a failed report fetch (fixed on this branch):** `try_collect().await?` returned early before awaiting the spawned download processor. The fetch result is now retained, the processor is always joined, and only then is the fetch error propagated. A focused async test verifies that the processor has completed before a fetch error returns.
- **DRY/documentation:** Consolidated the fetcher logging rule and corrected concurrency/failure documentation in `md/FETCHER.md`.
- Historical findings were checked against current code before being considered: current Tasker task errors are propagated with `bail!`, and the CRM download processor reports exhausted download retries as errors. These are not re-reported as open defects.
- **Unbounded download queue — assessed, no change yet:** the queue can accumulate tasks, but naively switching to bounded `mpsc` plus awaited sends is unsafe with the current `try_for_each_concurrent` processor: the processor exits on its first unrecoverable download error, which could stop draining the queue and leave producers blocked on `send().await`. Keep the current channel until the processor is changed to drain queued tasks while recording a failure and a behavioral backpressure test can verify no deadlock/lost task. This remains a resource-risk candidate, not a confirmed production defect.

## Additional repository-wide findings to prioritize after the current four items
- **P1 candidate — YASWEB JavaScript result parsing can report false success:** `src/yasweb/browser/javascript.rs::evaluate_automation_step` uses `serde_json::from_str(s).unwrap_or_default()`, then treats every status other than the exact string `ERROR` as success. Malformed JSON or a valid JSON object missing/invalid `status` can therefore be logged as successful automation. This contradicts `md/YASWEB_DEV.md`'s stated contract that automation steps must return a valid `SUCCESS` or `ERROR` result. Confirm all callers and add focused tests before fixing.
- **P1 security candidate — signed download URLs can leak to persistent logs/errors:** `src/crm/downloader.rs::download_csv` logs the full signed URL at INFO and includes it in GET transport errors. Signed URLs may be bearer-style access credentials. Redact URL query parameters and avoid including raw URL/body content in errors; add tests that verify sensitive query values never appear.
- **Documentation drift (P2):** `md/ARCHITECTURE.md` says shared CSV readers use `.flexible(true)`, but current `src/utils.rs::build_csv_reader_builder` does not enable flexible parsing and has a strict-width regression test. Update the stale statement when the relevant docs are next touched; no runtime change is needed.
- These are findings from the continuing scan, not included in the two current fetcher fixes. They remain unimplemented in this pass so the four requested items can be reviewed distinctly.

## Current verification status
- A draft PR was opened to run the repository's pull-request CI: [PR #410](https://github.com/Mahmoudiismail/crm/pull/410).
- The workflow was previously waiting at the `Format and Auto-fix` job's approval gate. The current branch now has additional fetcher changes and new async regression tests; check the latest run before claiming verification.
- No tests, formatting, Clippy, or release build results have been claimed as passed in this audit update.

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
