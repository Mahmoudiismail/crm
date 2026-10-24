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


## Follow-up: Clippy field-reassignment lint

### Finding
The CI Clippy job reports `clippy::field-reassign-with-default` as an error because the test initializes `RunnerConfig::default()` and then assigns `allow_shell_tasks = true` in a separate statement. The workflow uses `-D warnings`, so this lint fails Clippy.

### Minimal fix
Use a struct update initializer instead:
```rust
let mut cfg = RunnerConfig {
    allow_shell_tasks: true,
    ..RunnerConfig::default()
};
```
This preserves all other defaults and keeps shell execution enabled only in this isolated test configuration. Update the relevant operations documentation if needed. Do not change production defaults or the test invariant.

### Verification
The repository requires INTERNAL Code Review before tests. That capability is not available in the current environment, so do not claim review or run tests here. Confirm the updated Clippy and Test Suite CI results on PR #402 after the change.


### Follow-up from CI logs
The first struct-update correction removed the Clippy field-reassignment lint, but CI then reported Rust error E0596 at `cfg.tasks.push(task)` because the binding was immutable. Keep the struct-update initializer and declare the binding `mut`; this is necessary because the test appends its task to the config. The uploaded run shows both Clippy and Test Suite failed at compilation on this same E0596 error, before the test suite could execute.
