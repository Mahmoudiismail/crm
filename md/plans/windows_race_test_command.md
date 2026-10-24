# Windows Race-Test Command Fix

## Goal
Resolve the Windows CI failure in `test_real_duplicate_admission_race` where the test's execution marker is not created.

## Scope
- Change only the Windows shell command used by this existing regression test.
- Replace the PowerShell marker-writing command with a `cmd.exe /C` command that appends the marker and keeps the process alive briefly.
- Preserve the marker path, the non-Windows command, production code, and the existing execution-count assertion.
- Add no tests and no dependencies.

## Verification
- Run the repository-required INTERNAL Code Review before running tests.
- Then run `cargo fmt -- --check` and rerun the existing Windows CI test workflow.
- Do not claim verification passed unless it was actually executed.
- If the required INTERNAL Code Review capability is unavailable, stop before test execution and report the blocker.

## Constraints
- Keep the change minimal and test-only.
- Do not use `git reset`.
