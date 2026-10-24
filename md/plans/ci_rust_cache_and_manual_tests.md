# CI Cache and Manual Test Workflow Plan

## Goal
Reduce repeated Rust dependency compilation in GitHub Actions and provide a supported way to manually run the existing test workflow for verification.

## Scope
1. Give the Clippy and test jobs separate Rust cache keys so each job can restore and save its own target artifacts instead of competing for the same cache key.
2. Preserve the existing Rust cache action and toolchain setup; do not add dependencies or a new caching framework.
3. Add `workflow_dispatch` to `.github/workflows/test.yml` so the workflow can be launched manually from GitHub Actions.
4. Keep the existing `workflow_run` gate: automatically run Clippy and tests only after the `Auto Format` workflow succeeds.
5. Make checkout ref selection work for both `workflow_run` and manual dispatch.
6. Update `md/OPERATIONS.md` with cache behavior and manual run instructions.

## Constraints
- Smallest correct change; no architecture or unrelated CI redesign.
- Follow `AGENTS.md` and `md/AI_DOC_POLICY.md`.
- Do not use `git reset`.
- Do not claim a workflow or test passed unless its run result is available.
- The repository's mandatory INTERNAL Code Review capability is not available in the current toolset; report this limitation rather than substituting a manual review.

## Verification
- Inspect the complete workflow and documentation diff.
- Validate YAML structure and expressions where tooling is available.
- Trigger the GitHub Actions workflow if the available GitHub capability permits it; otherwise provide the exact manual path and report that execution could not be initiated from this environment.
- Verify the resulting workflow run and job statuses when available.
