# Rust CI Job-Scoped Cache Fix

## Goal
Allow Clippy and Test Suite to restore compatible Rust dependency artifacts while each job saves its own updated cache, instead of sharing one immutable exact cache key.

## Findings from the latest Windows CI logs
- Both jobs resolve the same cache key because `shared-key: rust-ci` deliberately removes the normal job-specific cache key.
- Both jobs report `Cache up-to-date` after execution. GitHub Actions caches are immutable; a successful exact hit is not replaced with the newly compiled target contents.
- `cache-on-failure: true` allows saving after a failed command but does not update an existing cache key.
- The Test Suite restored the cache and completed compilation, but its post-cache step did not create a new cache.

## Scope
- Replace the shared key with an additional `key: rust-ci`, allowing the action's default job-specific key to distinguish Clippy and Test Suite caches while retaining a common namespace.
- Keep `cache-on-failure: true` for the Test Suite so useful compiled dependency artifacts can be saved even if tests fail.
- Update `md/OPERATIONS.md` to describe job-scoped cache behavior accurately.
- No source-code or test changes are included in this cache-only fix.

## Verification
- Inspect the resulting workflow and documentation diff.
- Confirm subsequent CI logs show distinct cache keys for Clippy and Test Suite, and that the Test Suite creates/saves its own cache on a cache miss or partial hit.
- The repository requires INTERNAL Code Review before tests and a final review after implementation. That capability is not exposed in the current tool environment; do not claim the review occurred or independently run tests in violation of repository policy. GitHub Actions can validate the workflow after the commit.

## Constraints
- Keep the change minimal and avoid new dependencies or cache infrastructure.
- Preserve sequential job ordering and `cache-on-failure: true`.
- Do not use `git reset`.
