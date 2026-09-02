# Runner Scheduling & Dynamic-Date Improvements

## Problem Statement
The user requested three sets of changes to the Runner's scheduling configuration and dynamic date parsing to make default schedule assignments predictable and extend date evaluation capabilities, explicitly:
1. **Remove Interval Default**: Newly created tasks automatically default to an implicit 1-hour interval, which causes unintended scheduling behavior.
2. **Next Weekday / Beginning of Month Expressions**: Provide relative dynamic dates for `next <weekday>` (e.g. `next sat`) and `beginning_of_month` (first day of the month), while keeping `this_month` as a backward compatible alias.
3. **Sequential Dynamic Date Evaluation**: Dates configured recursively (such as an `eomonth` end-date that depends on a dynamic `start_date`) need to correctly resolve utilizing the generated start date as their base date. Furthermore, date resolutions must be validated (`start_date <= end_date`).

## Root Cause
- `forms.rs` and `models.rs` populated defaults explicitly selecting `1h` and substituting empty config parsing fields with `3600` frequency instead of leaving the interval disabled or empty (`Vec::new()`).
- `utils.rs` dynamic date evaluation (`resolve_date_var`) was missing handlers for weekday increments and explicit month-starts, restricting date parameterization options.
- The configuration loaders lacked a dependency passing pipeline whereby a dynamically evaluated start date became the specific `base_date` for end-date parsing, meaning they parsed orthogonally and resolved incorrectly.

## Implementation Details
1. **Manual Schedule GUI/Backend Fix**:
    - Modified GUI template scripts (`forms.js` and `templates.rs`) to default new schedules to empty rather than explicitly seeding them with `1h`.
    - Modified `forms.rs` so that if no `schedules` form-data is found, the legacy variables map to `0` interval (`Manual`) instead of implicitly assuming `3600`.
2. **Dynamic Date Extensibility**:
    - Handled `beginning_of_month` in `utils.rs`, utilizing the same calculation as `this_month` (which was preserved).
    - Handled `next mon` ... `next sun` strings by resolving `chrono::Weekday` logic ensuring the retrieved day is strictly *after* the local/base date evaluated.
3. **Sequential Date Flow**:
    - Examined consumer workflows (`yasweb.rs`, `crm/mod.rs`) and confirmed the standard flow performs sequentially: `let end_date = replace_date_vars(&e, start_date.as_deref())`.
    - Integrated runtime configuration assertions inside `crm/mod.rs` to validate if `from_date` exceeds `to_date` post-resolution, mirroring preexisting validation behaviors inside Yasweb.

## Tests Performed
- Validated regression tests via `cargo test --workspace`.
- Created unit tests confirming date logic for edge cases (e.g. Leap Year `eomonth` evaluations mapping dynamically against `beginning_of_month` offsets).
- Created tests tracking empty schedule deserialization mapping correctly into manual runner task persistence.
