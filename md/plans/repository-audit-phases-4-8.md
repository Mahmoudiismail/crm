# Repository Audit Plan — Phases 4–8

## Goal
Perform one evidence-based production audit of the current `main` baseline covering Phase 4 (Async & Concurrency), Phase 5 (Testing & Benchmarks), Phase 6 (API & Architecture), Phase 7 (Security & Filesystem), and Phase 8 (Documentation & Developer Experience). Keep the changes minimal and production-focused.

## Constraints
- This repository is a Cargo package with multiple binaries, not a Cargo workspace.
- Primary runtime platform is Windows.
- No enterprise architecture, unnecessary abstractions, framework-building, broad public API changes, or test scaffolding for its own sake.
- Historical audit reports are leads only; validate every finding against current source before calling it a defect.
- Do not change ZIP/release secrets or make unrelated release/policy changes.
- Do not merge the PR automatically. Keep all work in one branch and one PR.
- Update relevant `md/` documentation whenever runtime behavior changes.
- No unsupported claims about tests, benchmarks, or CI.

## Audit approach

### 1. Async & concurrency
- Trace process/task lifecycles, shutdown, cancellation, timeout behavior, and detached `tokio::spawn` tasks across Runner, CRM Fetcher, Yasweb, and Tasker.
- Inspect synchronous filesystem/process/sleep operations called from async execution paths and distinguish genuinely blocking work from bounded/low-risk operations.
- Check mutex/lock scope, semaphore/queue limits, child-process cleanup, stdout/stderr draining, and cancellation behavior.
- Only change code where the current execution path demonstrates a correctness, resource-lifecycle, or responsiveness defect.

### 2. Testing & benchmarks
- Inventory existing unit/integration tests and Criterion benchmarks before adding tests.
- For every accepted behavior fix, add focused regression coverage for the actual invariant/failure mode, preferably behavioral tests for concurrency, subprocess, filesystem, and security behavior.
- Reuse existing test conventions; do not introduce a new test framework or broad dependency-injection architecture.
- Run formatting, strict Clippy, tests, and relevant benchmarks if the available environment permits; otherwise report exact limitations and rely on CI without claiming results prematurely.

### 3. API & architecture
- Review boundaries between binaries/modules, input/output contracts, validation and error propagation, and duplicated logic.
- Avoid cosmetic refactors. Only simplify or consolidate code when it removes a demonstrated defect or meaningful maintenance risk without expanding public API unnecessarily.

### 4. Security & filesystem
- Audit path construction and validation, temporary-file handling, atomic writes/replacements, download destination safety, malformed inputs, secret persistence/logging, and dynamic PowerShell/command argument handling.
- Validate trust boundaries and exploitability before assigning severity.
- Add regression tests for confirmed security/reliability fixes. Do not rewrite shared history or rotate external credentials as part of this PR.

### 5. Documentation & developer experience
- Check impacted `md/` pages against actual code and commands.
- Update only documentation affected by accepted changes; remove stale claims only when source evidence is clear.
- Review test/CI instructions for accuracy where touched.

## Expected deliverables
- This plan committed first on the audit branch.
- A concise findings/remediation record separating confirmed issues, already-fixed/stale audit claims, and non-actionable risks.
- Minimal code changes with focused regression tests and relevant `md/` updates.
- One draft PR targeting `main`, with verification status stated accurately.

## Verification checklist
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets`
- Run relevant Criterion benchmark(s) only when a performance-related behavior is changed or evidence is needed.
- Inspect the full final diff for correctness, scope, documentation, and accidental changes.
- Confirm CI results on the PR before treating the work as verified.

## Explicit non-goals
- No wholesale rewrite of async orchestration.
- No blanket replacement of every `std::fs` call or every `.unwrap()` regardless of context.
- No claim that every historical audit concern is a vulnerability.
- No automatic merge.
