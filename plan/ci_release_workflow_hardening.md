# Plan: CI and Release Workflow Hardening

## Goal
Apply the approved, minimal repository optimizations to GitHub Actions without changing application behavior or introducing dependencies.

## Scope
1. Harden `.github/workflows/release.yml`:
   - Read the manually supplied binary selection through an environment variable rather than interpolating it into shell source.
   - Accept only the five remaining binary targets: `crm`, `runner`, `yasweb`, `tasker`, and `crm_updater`.
   - Reject empty selections, unknown names, and malformed comma-separated values.
   - Build and package using Bash arrays; remove dynamic command construction and `eval`.
   - Preserve the `all` selection and current release artifact naming.
2. Make the Clippy warning policy explicit in `.github/workflows/test.yml` by passing `-- -D warnings`.
3. Update `md/BUILD_AND_RUN.md` to reflect the actual CI commands and explain the allowed release binary selections.
4. Keep this plan in the PR as required by `AGENTS.md`.

## Out of Scope
- No application source changes, new dependencies, architecture changes, or broad test-framework changes.
- No modifications to PR #397 or the default branch.
- No benchmark-driven runtime changes in this PR; those require measured baselines and a separate focused change.

## Verification
- Inspect the full diff for shell injection paths, binary allowlist consistency, packaging parity, and documentation accuracy.
- Validate YAML and shell syntax where tooling is available.
- Run repository CI (format, Clippy with warnings denied, tests) if an executable checkout/toolchain is available. Do not claim checks passed unless they are actually run.
- Open a pull request against `main`; do not merge it automatically.
