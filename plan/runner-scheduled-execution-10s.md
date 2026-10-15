# Runner Scheduled Execution — 10-Second Regression Test Plan

## Goal
Verify that the Runner scheduler automatically executes an enabled task whose schedule becomes due shortly after the scheduler starts. The user reported that a manually configured future task did not run.

## Scope
- Runner only; no changes to other binaries or CI architecture.
- Keep the smallest correct V1 change.
- Do not merge to `main` without separate explicit authorization.
- Keep the test deterministic and avoid depending on the real wall clock where possible.

## Inspection findings
- `start_scheduler` polls on the configured interval, with a minimum of 5 seconds and a default of 30 seconds.
- The scheduler scans enabled tasks and their enabled schedules, then sends `RunTaskNow` when `schedule_is_due` returns true.
- The existing scheduler tests do not currently demonstrate end-to-end execution of a future scheduled task through the scheduler polling loop.

## Implementation after explicit START
1. Add a focused Runner regression test using a temporary config and an enabled task scheduled for approximately 10 seconds in the future.
2. Make the task produce a deterministic marker (temporary file) so the test can assert actual execution, not just schedule calculation.
3. Use a bounded wait long enough to cover the polling interval; assert the marker appears, and clean up temporary files.
4. Run the focused test on Windows CI, then run the complete Test Suite, Format and Auto-fix, and Clippy Lints.
5. If the test fails, trace the actual scheduling path and fix only the confirmed cause; do not weaken the assertion or arbitrarily increase production timeouts.

## Verification criteria
- The test passes on Windows CI and confirms the task executed automatically without a manual Run command.
- The full CI suite passes.
- Report actual CI results; do not claim a pass before observing it.

## Stop conditions
Do not implement until the user explicitly replies `START`. Do not merge without separate authorization.
