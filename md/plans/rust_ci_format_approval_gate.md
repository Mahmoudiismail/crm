# Require Approval Before Rust CI Formatting

## Goal
Require a human approval before the existing `Format and Auto-fix` job starts in `.github/workflows/test.yml`. Once approved, preserve the existing job order: format, Clippy, then tests.

## Implementation
- Add the GitHub Actions Environment `format-approval` to the existing `format` job.
- Do not change the current formatting commands, auto-commit behavior, job dependencies, or test conditions.
- Document the required one-time repository setup: create the `format-approval` environment in repository Settings and add the intended approver under Required reviewers.
- GitHub Environment protection rules are configured in repository settings, not created by the workflow YAML. The workflow can reference the environment, but repository settings must enable required reviewers for the approval gate to actually pause the job.

## Verification
- Inspect the workflow diff and confirm the environment is attached to the `format` job, not a later job.
- Confirm the existing dependency chain remains unchanged.
- Do not claim a live approval gate is active until the Environment's Required reviewers setting is configured in GitHub.
- CI verification is required after the change; the current environment does not expose the repository's required INTERNAL Code Review capability, so do not claim that review occurred.
