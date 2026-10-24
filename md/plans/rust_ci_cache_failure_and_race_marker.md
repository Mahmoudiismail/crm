# Rust CI Cache-on-Failure and Runner Race Marker

## Goal
Preserve useful Rust build cache data when CI tests fail and make the duplicate-admission race regression test reliably locate its execution marker on Windows and Unix.

## Scope
- Set `cache-on-failure: true` on both `Swatinem/rust-cache@v2` steps in `.github/workflows/test.yml`.
- Change `test_real_duplicate_admission_race` to create/read/remove its marker using an absolute path inside the test's temporary directory, rather than relying on the child process's working directory.
- Keep the test's real shell-command execution and exact-one-execution assertion; do not skip or weaken the concurrency invariant.
- Update `md/OPERATIONS.md` and this plan with the CI cache behavior and regression-test behavior.

## Verification
- Inspect the complete relevant source and workflow changes.
- Confirm cache configuration and the marker path are consistent across platforms by code inspection.
- Required INTERNAL Code Review must run before tests and again on the final diff. The current tool environment does not expose that capability, so do not claim it occurred or run tests in violation of repository policy.
- After the required review is available and findings are addressed, run the targeted race test, then the repository-required CI checks.

## Constraints
- Keep changes minimal; no new dependencies or architecture changes.
- Preserve the existing duplicate-admission behavior and test assertion.
- Do not use `git reset`.
