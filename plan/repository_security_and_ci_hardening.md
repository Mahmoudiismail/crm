# Repository Security and CI Hardening

## Scope
Implement the approved post-merge audit recommendations, explicitly excluding any change to the fixed ZIP password in `.github/workflows/release.yml`.

## Work plan
1. Remove tracked operational data files under `TestingDownloads/` from the current tree, and add a short README explaining that real exports/configs must remain local and that only synthetic fixtures belong in version control. Update `.gitignore` with explicit protection for the directory and common export/config patterns. Review other tracked files for obvious personal or operational data before concluding this scope.
2. Make every workflow capable of formatting and pushing Rust changes use the same `format-approval` environment, including `.github/workflows/auto-format.yml`. Keep the existing format gate in `test.yml`.
3. Harden release idempotency without changing the ZIP password: prevent publishing over an existing version tag/release and validate the source/tag state before creating release artifacts. Keep the currently configured archive password untouched.
4. Review current `deny.toml` RustSec ignores and dependency metadata. Do not blindly remove ignores without confirming their current applicability; document each unresolved advisory and update `md/DEPENDENCY_AUDIT.md` to state verified facts and the date/limits of the audit.
5. Document all operational changes under `md/`, inspect the full diff, and open a PR targeting `main`.

## Git history / exposure response
Deleting files from the current tree does not remove historical versions from Git history. Do not rewrite shared `main` history automatically. Document a coordinated history-cleanup procedure and assess whether exposed credentials need rotation. Any history rewrite is a separate, explicit operation because it changes commit IDs and affects existing clones/forks.

## Verification and constraints
- Inspect the workflow YAML and release behavior changes.
- Run applicable CI only after the repository-mandated INTERNAL Code Review.
- The INTERNAL Code Review capability is not currently exposed in this environment; do not claim it was run or run tests as a substitute.
- Preserve the ZIP password exactly as requested.
- Do not use `git reset`.


## Execution status (October 14, 2026)

### Implemented on the branch
- Removed the tracked contents of `TestingDownloads/` and retained only its README policy. Updated `.gitignore` so local-only data in that directory is not re-added accidentally.
- Added the `format-approval` Environment to both the main test workflow's format job and the separate manual auto-format workflow.
- Updated the release workflow to resolve `main` once, build from that exact SHA, fail closed if remote tag lookup fails, reject an existing version tag, serialize releases by version, and avoid overwriting release assets. The ZIP password remains exactly `123456`, as explicitly requested.
- Removed active quick-xml vulnerability suppressions and withdrawn 2024 advisory ignores from `deny.toml`; retained only documented informational unmaintained-crate exceptions.
- Updated `Cargo.lock` from `wayland-scanner 0.31.10` to `0.31.11`, which uses `quick-xml ^0.41`; the lockfile no longer contains `quick-xml 0.39.4`.
- Updated the test workflow path filters and the operational/dependency audit docs.

### Outstanding / blocked
- Cargo has not validated the manually updated lockfile yet. Required next checks are `cargo check --locked`, `cargo tree -i quick-xml`, `cargo audit`, and `cargo deny check advisories`.
- The required INTERNAL Code Review capability is not exposed in this environment. Do not run or claim verification until the required review can be performed. No manual self-review is being represented as an equivalent.
- Current-tree deletion does not remove data from Git history, forks, cached archives, or existing clones. History rewriting has not been performed because it changes commit IDs and needs a coordinated explicit operation. Assess exposure and rotate any credentials found.
- Keep PR #404 in draft until the required review and verification are complete.


## Follow-up: restore test independence after export removal

- [x] Replace the dashboard updater and CRM Open Sohail test helpers' dependency on `TestingDownloads/` with synthetic CSV inputs and an inline minimal tasker configuration.
- [x] Document the synthetic-fixture requirement in `md/TESTING.md`.
- [ ] Verify the commit with GitHub Actions; do not claim test success until the workflow completes.
