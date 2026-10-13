# Runner Race Marker Command Fix

## Finding
The race regression test currently constructs a compound `cmd.exe /C` command. The runner's `run_shell_command` does not pass the raw command string directly to a shell: it first tokenizes it with `parse_command_args`, then launches non-builtin executables directly. The nested `cmd.exe /C "echo ... & timeout ..."` therefore relies on multiple layers of quote handling and cmd.exe `/C` parsing, making the marker write unreliable. The test's missing marker shows the current command did not produce the expected file.

## Scope
- Keep the real duplicate-admission concurrency test and exactly-one-execution invariant.
- Replace the Windows compound `cmd.exe` command with a single `powershell.exe -NoProfile -NonInteractive -Command` invocation that uses `Add-Content -LiteralPath` to write the marker and `Start-Sleep` to keep the process alive long enough for both admissions to race.
- Escape single quotes in the literal path if necessary, so the PowerShell string remains valid.
- Keep the Unix command unchanged unless review reveals an equivalent issue.
- Update `md/OPERATIONS.md` to explain the marker's cross-platform behavior.

## Verification
- Inspect `parse_command_args`, `run_shell_command`, and the regression test together to ensure the PowerShell script is passed as one argument and the path is quoted safely.
- Repository policy requires INTERNAL Code Review before tests and a final review after implementation. That capability is unavailable in the current environment; do not claim it occurred or run tests in violation of policy.
- Once the required review is available, run the targeted race test on Windows, then the full CI suite.

## Constraints
- Minimal test-only behavior change; no dependencies or production execution changes.
- Preserve exactly-one-execution assertion.
- Do not use `git reset`.
