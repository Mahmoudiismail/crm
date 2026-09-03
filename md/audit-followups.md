# Audit Follow-ups

## Finding F-001 (Runner Pipeline Reliability)

**Original Audit Claim:**
The post-merge audit (#308) identified a HIGH-priority reliability concern in `src/runner/engine/pipeline.rs`, specifically noting unsafe `.unwrap()` calls around lines 260, 310, and 372. The audit claimed this could cause the entire runner daemon to panic and crash if process output piping or state tracking encountered an unexpected `None` or error.

**Investigation Results:**
- **Actual Location:** The `unwrap()` calls on lines 260, 310, and 372 are strictly contained within the `#[cfg(test)] mod tests` block.
- **Production Code:** An inspection of the production pipeline code (`execute_step`, `execute_pipeline`, `run_task_inner`, and related process modules like `process.rs`, `application.rs`, and `shell.rs`) revealed no occurrences of `unwrap()`, `expect()`, `panic!()`, or unchecked indexing.
- **Error Propagation:** The production pipeline correctly propagates errors. For example, Tokio `JoinError`s during parallel execution are handled using `handle.await.context("...")?`. Child-process failures are similarly propagated via `Result` and `anyhow::Context`. `run_task_inner` explicitly handles execution errors without panicking.
- **Resource/Lifecycle:** Process failures and timeouts are cleanly managed, ensuring child processes are reaped correctly and the daemon continues running.

**Final Classification: FALSE POSITIVE**
The reported unwrap() calls are test-only and pose no risk to the production daemon. The production pipeline already safely handles and propagates errors.

**Action Taken:**
No production code changes were required or made. This finding is closed.

## Tasker Task 3, 4, 5 Reliability Fixes (CRM-Open-Sohail, Department Split, OPD)

### Task 3 (CrmOpenSohail)
**Problem:** The COM application crashed during execution due to an inability to locate the `http://schemas.microsoft.com/mapi/proptag/0x39FE001E` property, blocking draft creation. The `subject` string was not properly injected, and `.To` / `.CC` were being unnecessarily manipulated.
**Fix:**
- Updated the PowerShell generator to strictly use `chrono::Local::now() - 1 day` to format the `"Open TKTs DD-MMMM"` subject inside Rust, and passed this exact string to PowerShell.
- Removed the appended `email_to` and `email_cc` arrays entirely so `.ReplyAll()` perfectly respects the original list without mutations.
- Re-architected sender lookup to natively utilize `$ExchangeUser.PrimarySmtpAddress` wrapped in `try/catch`, falling back to `PropertyAccessor` in a `try/catch` and gracefully failing matching if both are unavailable, rather than halting execution.

### Task 4 (DepartmentSplit)
**Problem:** PowerShell parsing failed with `InvalidVariableReferenceWithDrive` due to a string interpolation collision on `$target: $targetPath`.
**Fix:**
- Fixed the interpolation inside the Rust generator by double-wrapping the Rust formatting blocks to literally emit `${target}: ${targetPath}` inside PowerShell, unambiguously protecting the variable definition from adjacent punctuation.
- Added extensive `TRACE` logging to output exactly which iteration failed if an exception is thrown in the block.

### Task 5 (OPD Analysis)
**Problem:** The AutoFilter dropdown icon remained visible in the saved Excel screenshot outputs.
**Fix:**
- Rewrote the PowerShell logic so it loops from `1..Columns.Count` and executes `$exactRange.AutoFilter($col, [Type]::Missing, 1, [Type]::Missing, $false)` dynamically, strictly hiding only the `VisibleDropDown` icons while retaining the actual data filtering.
