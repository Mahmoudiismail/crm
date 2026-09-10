# Plan: Add Unit Tests for `next_monthly_run_after`

## Overview
The function `next_monthly_run_after` in `src/runner/config/schedule.rs` handles non-trivial date arithmetic, timezone conversions, month-end day clamping (e.g., day 31 in 28/29/30-day months), year boundary crossing, time string parsing, and working hours filtering. Currently, it lacks unit test coverage. This task adds comprehensive test cases covering happy paths, edge cases, and error conditions for `next_monthly_run_after`.

## Proposed Plan

1. **Create Plan Document**:
   - Write this plan file to `plan/add_tests_for_next_monthly_run_after.md` as required by AGENTS.md.

2. **Implement Unit Tests**:
   - Add unit test function `test_next_monthly_run_after` inside `src/runner/config/schedule.rs` under `mod tests`.
   - Test cases to cover:
     - **Same Month, Future Time**: Test when target day/time is later in current month.
     - **Same Month, Past Time**: Test when target day/time in current month has passed; verifies advance to next month.
     - **Month End Clamping**: Test day_of_month = 31 in February non-leap year (clamped to 28), leap year (clamped to 29), and April (clamped to 30).
     - **Year Boundary Crossing**: Test execution from December advancing to January of next year.
     - **Time Parsing & Defaults**: Empty string `at_time` (defaulting to 00:00:00) vs explicit `HH:MM`.
     - **Invalid Time Handling**: Returning `Err` for invalid time strings (e.g. "25:00", "invalid").
     - **Working Hours Filtering**: Test with `working_hours` map to ensure candidate dates falling on non-working days are skipped until a valid working day is found.

3. **Verify Tests**:
   - Run `cargo test --lib runner::config::schedule` to confirm new unit tests pass.
   - Run full test suite `cargo test` to confirm zero regressions across the crate.

4. **Pre-commit Steps & Submission**:
   - Run pre-commit checks.
   - Commit changes and submit.
