# Repository Reliability and Performance Remediation Plan

## Baseline and working rules
- Base: `main` at `86806fbd07f61b794d1a819c24f165c501a1792f` (PR #455 merge).
- Work on one branch and submit one PR only: `fix/repository-reliability-optimization`.
- Preserve V1 scope: smallest correct changes, no new framework or unnecessary public API/dependency.
- Add a focused regression test for each behavior change and update the relevant `md/` documentation.
- Do not merge the PR without a separate explicit instruction.
- Re-check source on this base before changing any item below; if an audit observation is not present or is already fixed, record that and do not force a redundant change.

## Phase 1 — Yasweb correctness
1. Protect the full `yasweb_config.json` read-modify-write operation across processes, including both discovered-filter persistence and CLI-driven config updates. Use the existing locking/dependency facilities where suitable; retain atomic replacement and avoid lock-file lifecycle races.
2. Prevent filter option values from unrelated reports being combined. Keep a list control only when the applicable reports have compatible option sets; otherwise expose a safe free-text input without changing the public manifest contract unnecessarily.
3. Keep finalization lock artifacts out of report output directories. Use a stable lock location and preserve cross-process serialization; never unlink a lock inode while another process may be waiting on it.

Required evidence: regression tests for simultaneous independent config writers, conflicting report filter values, and finalization output-directory cleanliness/concurrent finalization.

## Phase 2 — Runner correctness and local GUI security
4. Preserve a valid stored monthly schedule `next_run_at` during task normalization/editing; only initialize or recompute it when missing/invalid or when schedule-defining fields genuinely change.
5. Validate mutation requests to the loopback GUI against the request origin/host using the existing HTTP parser. Preserve legitimate local/CLI requests that intentionally omit Origin, and reject cross-origin browser form submissions. Check Host/DNS-rebinding implications before settling the minimal policy.

Required evidence: schedule regression tests for due/overdue persisted tasks and GUI request tests for same-origin, absent-Origin, and hostile cross-origin requests. If the request parser cannot reliably support the intended security invariant, document the limitation and do not claim the CSRF issue fixed.

## Phase 3 — Tasker correctness, paths, and data handling
6. Resolve Department Split configured paths consistently with the executable-relative path convention used by the other Tasker tasks.
7. Guard OPD/Sat Figures arithmetic against booked-slot counts exceeding total slots; return a clear validation error rather than panic/wrap or silently producing negative/invalid totals.
8. Inspect Tasker config validation and add only compatibility-safe validation for settings that can cause invalid ranges, overflow, or unsafe processing. Do not reject legacy configurations without evidence.
9. Remove sensitive raw ticket/customer content and recipient addresses from diagnostic logs while retaining actionable file/row/error context.

Required evidence: changed-working-directory path test, malformed OPD arithmetic tests, config validation tests for accepted legacy values and rejected invalid values, and logging tests or review demonstrating sensitive payloads are not emitted.

## Phase 4 — Evidence-based performance work across all binaries
10. Evaluate streaming Tasker users-export and lead-report CSV processing to reduce peak memory without changing row ordering, delimiter detection, BOM handling, strict CSV record-width validation, or diagnostics. Implement only where the current code and tests support a behavior-preserving improvement.
11. Evaluate loading Runner configuration once per scheduler polling cycle and reusing manifest discovery within a single external-app execution. Preserve current cleanup, execution, and cache invalidation semantics.
12. Review CRM, Yasweb, Runner, Tasker, and updater paths for additional measurable/high-confidence hot spots. Do not make speculative micro-optimizations; record candidates not justified by evidence.

Required evidence: focused correctness tests plus existing test suite; use benchmarks or allocation/throughput measurements when a performance claim is made. Keep optimization changes separate in the review narrative from correctness/security fixes.

## Explicitly deferred security decision
- CRM Updater update-package authenticity remains a production security concern: a checksum is not authenticity unless its expected value is trusted. Do not introduce a signing key, trust root, key rotation, or bootstrap design in this PR without an explicit product/security decision. Record the limitation in the final audit/report and point to the existing production-readiness documentation.

## Documentation
- Update the relevant files under `md/` for every behavior change.
- Update the repository audit/remediation record with confirmed findings, applied fixes, deferred items, and verification evidence.
- Keep operational instructions and examples aligned with actual behavior.

## Verification and acceptance
1. Run `cargo fmt --all -- --check`.
2. Run focused tests for each changed subsystem, then `cargo test --all-targets` (or document any target/environment limitation).
3. Run `cargo clippy --all-targets --all-features -- -D warnings` with the repository's actual supported feature/target configuration; do not weaken lints to obtain a pass.
4. Verify Windows CI workflows, inspect every job and failed log, and rerun affected checks after any fix.
5. Manually review the complete final diff against this plan and the original request. No claims of runtime verification for browser/Windows COM behavior unless actually exercised.
6. Open exactly one PR targeting `main`; do not merge it.
