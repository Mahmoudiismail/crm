# Runner Race Marker Test Fix

## Root cause
`RunnerConfig::default()` sets `allow_shell_tasks` to `false`. The real duplicate-admission race test creates its config with this default and never enables shell tasks, so the Runner correctly rejects the test's ShellCommand action before the marker command can run. The subsequent missing marker is a consequence of test setup, not proof of a Windows path or quoting failure.

## Scope
- Explicitly set `cfg.allow_shell_tasks = true` in this test's isolated RunnerConfig so its ShellCommand action is permitted.
- Keep the test's real concurrent admission requests and exactly-one-execution assertion.
- Retain the absolute marker path and the Windows PowerShell marker command, which avoid relying on a relative working directory and nested `cmd.exe /C` quoting.
- Update `md/OPERATIONS.md` to document the actual cause and test setup.

## Verification
- Code inspection confirms `default_allow_shell_tasks()` returns `false` and the test uses `RunnerConfig::default()`; the test must explicitly enable shell commands to exercise the behavior it intends to test.
- The INTERNAL Code Review capability required by repository policy is not exposed in the current environment. Do not claim it occurred or run tests in violation of the policy.
- After required review, run `test_real_duplicate_admission_race` on Windows, then the complete CI suite.

## Constraints
- Keep the change limited to test setup and its diagnostics.
- Do not weaken the exactly-one-execution invariant or alter production shell-task security defaults.
- Do not use `git reset`.
