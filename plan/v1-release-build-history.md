# V1 Release Build History

## Goal
Automatically append a durable CSV record for each manual invocation of the Release Binaries workflow, including attempts that fail before compilation begins.

## Changes
- Add `plan/v1-release-build-history.csv` with the UTC timestamp, run ID/attempt, source SHA, package version, selected binaries, build status, release status, and failure reason.
- Update `.github/workflows/release.yml` to capture the start timestamp, summarize tag-guard/build/publish outcomes, and append one history row from an `always()`-conditioned job.
- Serialize release workflow runs to prevent concurrent history commits from racing.
- Document the file and status semantics in `md/RELEASE_BUILD_HISTORY.md`.

## Constraints
- Do not change application binaries or release-tag protection.
- Existing version tags must continue to block publication without being misreported as compilation failures.
- Keep changes limited to release workflow tracking, its history file, documentation, and this plan.

## Verification
- Review the workflow's dependency conditions, status outputs, append-only CSV handling, and push behavior.
- Confirm the final PR diff contains only the intended files.
- Runtime verification still requires a GitHub Actions run; do not claim it passed without one.
