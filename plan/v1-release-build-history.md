# V1 Release Build History

## Goal
Persist a repository-visible CSV history for each release build attempt initiated by the `Release Binaries` GitHub Actions workflow.

## Scope
- Add `plan/v1-release-build-history.csv` as the append-only history file.
- Update `.github/workflows/release.yml` to record an attempt before the release-tag guard and update the same row after the job finishes.
- Serialize release workflow runs to prevent concurrent updates from overwriting history.
- Document the operational behavior in `md/BUILD_AND_RUN.md`.

## Recorded fields
UTC start timestamp, workflow run ID and attempt, source commit SHA, package version, selected binaries, build status, release status, and failure reason.

## Status semantics
- A tag conflict is a release/pre-build guard failure; it must not be reported as a compilation failure.
- If the build step never starts, build status is `not_started`.
- If publication never starts, release status is `not_started`.
- The finalizer runs even when prior release steps fail, and updates the row created at the start.

## Verification
- Review YAML structure, step IDs/conditions, CSV header and matching run identifiers.
- Inspect the final Git diff and GitHub Actions results after the PR is opened.
- Do not claim an actual release workflow execution has passed unless one is run and succeeds.
