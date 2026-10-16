# Plan: Fix Runner periodization integration tests on Windows

## Scope
Fix only the three failures in PR #422 (`fix/runner-tasker-date-args`) from workflow run 38975844566. Do not modify PR #423 or merge any branch.

## Findings
- `test_multiple_external_apps_execution_isolation` and `test_post_run_external_app_execution` assert that dates appear in the child process arguments but leave `start_date_arg` and `end_date_arg` unset. The current contract intentionally avoids injecting generic date flags when the app has not declared or mapped them.
- `test_concurrent_period_execution_overlapping` also leaves the date mappings unset and fails at `res.success`; its failure must be checked after the test process is made deterministic for Windows.
- These tests use `python3` as the executable on `windows-latest`, while the workflow does not explicitly install Python. That executable name is not a portable Windows assumption.

## Implementation
1. Make the Python-based integration tests use a deterministic Windows-available Python executable provisioned by the test workflow.
2. In the three failing tests, set explicit `--start-date` and `--end-date` mappings because these tests are intended to verify propagation of mapped dates, not implicit generic flags.
3. Keep the production Runner behavior unchanged unless the rerun exposes a separate, reproducible runtime defect.
4. Update this plan and the relevant `md/` documentation if runtime/configuration behavior is changed. For a test-only correction, avoid unrelated documentation churn.

## Verification
- Run the three affected integration tests on the Windows CI runner.
- Run the complete integration test suite, formatting, and Clippy through PR CI.
- Review the final diff and confirm the change is limited to PR #422.
