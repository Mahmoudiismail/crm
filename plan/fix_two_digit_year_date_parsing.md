# Fix Two-Digit Year Parsing with Chrono

## Objective
Support parsing two-digit year dates (e.g., `'01-May-26'`, `'01 May 26'`, `'May 01, 26'`, `'01-05-26'`, `'01/05/26'`) in `parse_flexible_date_impl` in `src/utils.rs` using chrono's `%y` specifier.

## Proposed Changes

### `src/utils.rs`
1. Update `parse_flexible_date_impl` date formats slice to include two-digit year formats:
   - `"%d-%b-%y"` (e.g. `01-May-26`)
   - `"%d %b %y"` (e.g. `01 May 26`)
   - `"%b %d, %y"` (e.g. `May 01, 26`)
   - `"%d-%m-%y"` (e.g. `01-05-26`)
   - `"%d/%m/%y"` (e.g. `01/05/26`)
   - `"%y-%m-%d"` (e.g. `26-05-01`)
   - `"%y/%m/%d"` (e.g. `26/05/01`)

2. In `src/utils.rs` test module (`mod tests`):
   - Uncomment `assert_eq!(to_iso_date("01-May-26"), "2026-05-01");`.
   - Add additional test cases verifying two-digit year parsing formats.

3. Run `cargo test --lib utils::tests` to verify all date parsing tests pass.

## Documentation and Pre-commit Verification
1. Ensure the plan document `plan/fix_two_digit_year_date_parsing.md` is committed alongside the code changes.
2. Complete pre-commit verification steps.
