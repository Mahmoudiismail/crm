# Full Repository SOLID & DRY Audit — V1

## Status
Implementation authorized by the user's explicit **START**. Work is being performed on `audit/solid-dry-v1-oct-2026`; do not merge into `main` without separate authorization.

## Confirmed finding and remediation
- **Security/privacy — CRM response payload logging (fixed on this branch):** `src/crm/fetcher.rs` logged the full incomplete-reservation PATCH body and full CRM API response bodies at DEBUG; arbitrary response bodies were also embedded in several HTTP errors and malformed JSON contexts. Responses can contain signed download URLs, report data, and ticket IDs. The change now logs response metadata/byte count, omits PATCH body contents (logs only ticket count), and sanitizes HTTP/parse errors. It preserves the `failed to generate signed url` marker needed by recursive date-range splitting.
- **DRY/documentation:** Consolidated the fetcher logging rule into one policy section in `md/FETCHER.md`.
- Historical findings were checked against current code before being considered: current Tasker task errors are propagated with `bail!`, and the current CRM download processor is awaited and propagates download failures. These are not re-reported as open defects.
- **Candidate for continued review, not yet classified as a defect:** CRM report scheduling uses an unbounded MPSC queue for download tasks. Assess whether real date-range workloads can accumulate enough queued tasks to justify bounded backpressure; do not change it without a safe concurrency review and behavioral test.

## Current verification status
- A draft PR was opened to run the repository's pull-request CI: [PR #410](https://github.com/Mahmoudiismail/crm/pull/410).
- The workflow is currently waiting at the `Format and Auto-fix` job's approval gate. Formatting, tests, Clippy, and release build have **not** been claimed as passed. Continue after the required GitHub Environment approval and inspect the final workflow results.

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
