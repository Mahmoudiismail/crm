# Execution Plan: Update CI Test Workflow to use Release Mode

## Objective
Update the `cargo clippy` and `cargo test` commands in the GitHub Actions test workflow to run in release mode.

## Steps
1. **Update `.github/workflows/test.yml`**:
   - Locate the `Run clippy` step.
   - Append `--release` to the command: `cargo clippy --workspace --all-targets --all-features --release`.
   - Locate the `Run tests` step.
   - Append `--release` to the command: `cargo test --workspace --all-targets --all-features --release`.
2. **Review and Verify**:
   - Ensure that the syntax in `.github/workflows/test.yml` remains valid.
3. **Pre-commit Checks**:
   - Call `pre_commit_instructions` and follow the provided steps.
4. **Submit**:
   - Commit the changes and open a PR or submit as requested.