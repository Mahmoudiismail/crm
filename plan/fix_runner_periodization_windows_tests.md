# Plan: Fix Runner periodization integration tests

## Scope
Fix only the three failures in PR #422 (`fix/runner-tasker-date-args`) from workflow run 38975844566. Do not modify PR #423 or merge any branch.

## Findings
- The current Runner behavior intentionally avoids injecting generic date flags when an app has not declared or mapped them.
- `test_concurrent_period_execution_overlapping` explicitly searches `sys.argv` for `--start-date`; because its `ExternalAppSpec` leaves both date mappings unset, the child process exits with an error and `res.success` is false.
- `test_multiple_external_apps_execution_isolation` and `test_post_run_external_app_execution` similarly assert that date strings appear in child arguments while leaving the date mappings unset.
- These failures are test setup mismatches with the intended behavior, not evidence that production Runner logic should restore generic date-flag injection.

## Implementation
1. Set explicit `start_date_arg = Some("--start-date")` and `end_date_arg = Some("--end-date")` in the three failing integration-test specifications.
2. Leave production execution logic unchanged; these tests should exercise explicitly mapped date arguments.
3. Do not change unrelated tests, workflows, PRs, or runtime behavior.

## Verification
- Confirm the three test specs now explicitly map the dates they assert.
- Run the affected integration tests, then formatting, Clippy, and the full test suite through PR CI.
- Review the final diff and confirm the change is limited to PR #422.
