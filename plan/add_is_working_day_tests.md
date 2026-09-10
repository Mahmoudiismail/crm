# Execution Plan: Add Unit Tests for `is_working_day`

## Overview
The function `is_working_day` in `src/runner/config/schedule.rs` checks if a given `DateTime<Utc>` falls on a working day according to a `HashMap<String, WorkingHours>` configuration map. Currently, there are no dedicated unit tests covering this function.

This plan details the testing strategy to achieve thorough test coverage for `is_working_day`, including happy paths, edge cases, wrap-around ranges, and invalid configurations.

## Target Function Analysis
Function signature:
```rust
pub fn is_working_day(
    working_hours: &std::collections::HashMap<String, WorkingHours>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool
```

Logic highlights to test:
1. Empty `working_hours` map -> returns `true`.
2. Single day matching:
   - Case/abbreviation variations: `"Mon"`, `"Monday"`, `"mon"`.
   - Matching current day returns `true`.
   - Non-matching day returns `false`.
3. Range parsing (`"StartDay - EndDay"`):
   - Standard range (e.g., `"Mon-Fri"` where `start <= end`): returns `true` for days within [Mon, Fri], `false` for Sat/Sun.
   - Wrap-around range (e.g., `"Fri-Mon"` where `start > end`): returns `true` for Fri, Sat, Sun, Mon, and `false` for Tue, Wed, Thu.
   - Spaces around hyphen in range (e.g., `" Mon - Fri "`).
4. Multiple range/day keys in `working_hours`:
   - Returns `true` if any key matches the given day.
5. Invalid/unrecognized keys (e.g., `"InvalidDay"`, `"X-Y"`):
   - Returns `false` when `working_hours` is non-empty but contains only unmatched/invalid keys.

## Test Implementation Strategy
1. Location: `src/runner/config/schedule.rs` in `mod tests`.
2. Timezone handling:
   - Build test timestamps in local time using `chrono::Local.with_ymd_and_hms(...)` and convert to Utc (`with_timezone(&Utc)`).
   - This ensures `now.with_timezone(&chrono::Local)` inside `is_working_day` produces the expected local weekday regardless of the machine's local timezone.
3. Assertions:
   - Standard unit test cases exercising each scenario with `assert!` and `assert!(!...)`.

## Execution Steps
1. Create `plan/add_is_working_day_tests.md` (this file).
2. Add `test_is_working_day` unit test function to `src/runner/config/schedule.rs`.
3. Run `cargo test --lib` to verify all tests pass.
4. Complete pre-commit procedures.
5. Submit changes.
