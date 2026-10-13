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
- Updated the release workflow to build from `main`, serialize releases by version, reject an existing version tag, and avoid overwriting release assets. The ZIP password remains exactly `123456`, as explicitly requested.
- Updated the test workflow path filters so release workflow, `.gitignore`, and `deny.toml` changes trigger CI.
- Updated `md/OPERATIONS.md` and `md/DEPENDENCY_AUDIT.md` with the new operational policy and verified advisory findings.

### Outstanding / blocked
- The available GitHub write capability blocked the proposed `deny.toml` update. The branch still suppresses RUSTSEC-2026-0194 and RUSTSEC-2026-0195 despite `Cargo.lock` containing vulnerable `quick-xml 0.39.4`, and still lists withdrawn 2024 GTK advisories. These entries need cleanup and the quick-xml dependency path needs a separate, verified upgrade.
- The release tag-check step currently detects an existing tag but its current shell conditional does not distinguish a failed remote lookup from a missing tag. Strengthen this to fail closed before treating release publishing as complete.
- Current-tree deletion does not remove the data from Git history. History rewriting has not been performed; coordinate it separately with the repository owner and any clone/fork users.
- No tests or cargo audit were run because the required INTERNAL Code Review capability is unavailable. Do not treat this branch as verified until the required review is available and CI passes.
