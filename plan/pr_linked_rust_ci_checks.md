# Ordered Rust CI and Shared Cache

## Goal
Ensure formatting completes successfully before Clippy and tests begin, make all results PR checks, and make the Test Suite restore the cache saved by Clippy.

## Scope
- Put Format and Auto-fix, Clippy Lints, and Test Suite in `.github/workflows/test.yml`.
- Use job dependencies so Clippy starts only after formatting succeeds and Test Suite starts after Clippy completes, even if Clippy fails, provided formatting succeeded.
- Have the format job run `cargo fmt`, commit/push formatting changes if needed, and verify `cargo fmt -- --check`.
- Make downstream jobs checkout the PR branch ref after formatting, so they test any formatting commit created by the format job.
- Use the same Rust cache shared key for Clippy and tests, with Test Suite depending on Clippy so its runner can restore the cache Clippy saved at job completion.
- Keep Windows runners, all existing Clippy/test flags and commands, Rust-file/Cargo path filters, and manual workflow dispatch.
- Stop the standalone Auto Format workflow from automatically running on PRs, avoiding parallel format jobs and races; retain manual dispatch.
- Update `md/OPERATIONS.md` to document the serial order and cache behavior.
- No production-code changes or dependencies.

## Verification
- Inspect the full workflow and docs diff.
- Use the repository's required INTERNAL Code Review before running tests. If unavailable, disclose that limitation and do not claim equivalent review.
- Validate workflow YAML and trigger/job dependency configuration where tooling permits.
- Confirm the PR has format, Clippy, and Test Suite checks; inspect cache logs for cache-hit/restore evidence and ensure Test Suite starts only after Clippy finishes.
- Report actual CI outcomes only.

## Constraints
- Minimal CI changes; no unrelated refactoring.
- Do not use `git reset`.
