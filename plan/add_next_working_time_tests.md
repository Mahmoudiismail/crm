# Plan: Add Unit Tests for `next_working_time`

## Objective
Increase test coverage and reliability for `next_working_time` in `src/runner/config/schedule.rs:338`.

## Testing Gap Analysis
Currently, `next_working_time` in `src/runner/config/schedule.rs` lacks dedicated unit tests. `next_working_time` calculates the next valid working time by advancing minute-by-minute when `now` falls outside configured working hours.

## Proposed Test Cases
The unit tests in `src/runner/config/schedule.rs` under `mod tests` will cover:
1. **Already within working hours**: Returns `now` unchanged.
2. **Before working hours on a working day**: Advances `now` to the start time on the same working day.
3. **After working hours on a working day**: Advances `now` to the start time on the next working day.
4. **Weekend / Non-working days**: Advances `now` across non-working days (e.g. Saturday) to the start time on the next working day (e.g. Monday morning).
5. **Multi-day day ranges**: Handles day range strings like `"Mon-Fri"`.
6. **Overnight / Wrapped working hours**: Handles overnight shifts where `start > end` (e.g. 22:00 to 06:00).
7. **Empty working hours HashMap**: Returns `now` because empty working hours evaluate as within working hours.
8. **Unreachable / Invalid working hours fallback**: Returns original `now` if 14-day loop finishes without finding a valid working time.

## Execution Steps
1. Create `plan/add_next_working_time_tests.md` (this file).
2. Add comprehensive tests to `src/runner/config/schedule.rs`.
3. Run `cargo test` to verify all tests pass.
4. Run pre-commit checks.
5. Submit PR.
