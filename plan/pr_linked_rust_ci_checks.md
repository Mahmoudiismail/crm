# PR-Linked Rust CI Checks

## Goal
Make Clippy and Test Suite results appear as checks on each pull request instead of running only as a separate workflow after Auto Format on the default branch.

## Scope
- Change `.github/workflows/test.yml` to trigger on pull requests targeting `main` only when Rust source files, `Cargo.toml`, or `Cargo.lock` change, plus manual `workflow_dispatch`.
- Run the existing Clippy and test jobs against the pull request's checked-out commit.
- Remove the `workflow_run` dependency on Auto Format, which reports the test run against the default branch rather than as a PR check.
- Preserve Windows runners, Rust cache keys, existing commands, and warning-deny flags.
- Update `md/OPERATIONS.md` to explain the new trigger behavior, the relationship to Auto Format, and manual execution.
- Do not change application code or add dependencies.

## Verification
- Review the complete workflow and documentation diff before testing.
- Run the repository-required INTERNAL Code Review before tests; if unavailable, stop before test execution and disclose the limitation.
- Validate YAML structure and trigger filters where tooling permits.
- Confirm GitHub creates Clippy and Test Suite checks against the PR head SHA.
- Do not claim CI passed unless the corresponding run completes successfully.

## Constraints
- Keep the change minimal; no new CI framework or unrelated workflow refactor.
- Do not use `git reset`.
