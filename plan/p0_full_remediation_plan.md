# P0 Full Critical Remediation Plan (Track A + Track B Corrections)

## Overview
This plan establishes the execution, testing, review, and verification strategy for the P0 Remediation task across both Track A (PowerShell & ScriptManager) and Track B (Runner Execution, Concurrency, Process & HTTP), incorporating all 29 correction items.

---

## Track A: PowerShell + ScriptManager Corrections

### 1. Genuine Cross-Process ScriptManager Contention Test
- **File**: `tests/script_manager_concurrency.rs`
- **Strategy**:
  - Implement a dedicated worker mode triggered via `--worker-id` CLI argument.
  - Implement parent-controlled file-barrier IPC synchronization (`.ready_proc1`, `.ready_proc2`, `.go`) so both worker processes reach ScriptManager critical section at the exact same moment.
  - Competing workers pass different templates (`v1_proc1` vs `v2_proc2`) with fixed timestamp overrides (`2026-09-08_19-42-15`) to force version allocation contention under lock competition.
  - Strict assertions: valid JSON metadata, unique version files (`concurrent_script.ps1`, `concurrent_script_2026-09-08_19-42-15_1.ps1`), active script refers to a valid file, no lost updates, and zero temporary `.tmp` files.

### 2. Lock Scope in ScriptManager
- **File**: `src/tasker/script_manager.rs`
- **Strategy**:
  - The OS-level lock (`fs2::FileExt::lock_exclusive` on `<task_dir>/.task.lock`) is acquired BEFORE `.metadata.json` is read and held continuously through corrupted metadata recovery, fingerprint comparison, active script validation, version allocation, script creation (`.tmp` -> `rename`), and metadata update (`.tmp` -> `rename`).
  - Strict path safety validation in `is_valid_filename` rejecting path traversal (`..`), slashes (`/`, `\`), colons (`:`), trailing spaces/dots, Windows reserved device names (`CON`, `PRN`, `NUL`, `COM1-9`, `LPT1-9`), and untrimmed whitespace.

### 3. Behavioral PowerShell Injection Safety Test
- **File**: `src/tasker/script_manager.rs`
- **Strategy**:
  - Create a behavioral test passing hostile parameter values through `-HostileVal` CLI argument to a static script template that writes the value to disk using `Set-Content -LiteralPath $OutFile -Value $HostileVal -NoNewline`.
  - Pass hostile string literals containing single quotes, double quotes, semicolons, `$`, backticks, ampersands, pipes, CR/LF, parens, subexpressions `$(...)`, and redirection characters.
  - Require execution success, read the output file in Rust, and assert exact string equality (`received_val == hostile_val`), proving literal reception without command execution.

### 4. CRM Updater — 100% Static Parameterized PowerShell
- **File**: `src/crm_updater/update.rs`
- **Strategy**:
  - Refactor all PowerShell workflows (`download_update_zip_from_drafts()`, `generate_update_script()`, `execute_detached_powershell()`, `unblock_file()`) to use static parameterized script templates (`SCAN_DRAFTS_TEMPLATE`, `UPDATE_SCRIPT_TEMPLATE`) with parameters passed strictly via CLI arguments (`-LogPath`, `-DownloadsDir`, `-ParentPid`, `-ReplacementMapJson`).
  - Sort `apps_to_stop` deterministically by `process_name` prior to JSON payload serialization.
  - Preserve all updater semantics: PID wait, process path inspection, SHA-256 hash checks, autostart, detached process flags, and failure exit code 1.

### 5. Reply Email PowerShell Contract Verification
- **File**: `src/tasker/crm_open_sohail/mod.rs`
- **Strategy**:
  - Verify static parameterized template `ps_email_template` (`param([string]$SenderAccount, [string]$SubjectPrefix, [string]$Subject, [string]$HtmlBody)`).
  - Verify all variable bindings (`$Inbox`, `$SentFolder`, `Find-OriginalMessage`, `$matches`, `$script:OriginalMail`, `$TargetMail.ReplyAll()`, `$ReplyMail.HTMLBody`, `$ReplyMail.Save()`).
  - Unit test explicitly asserts `$ReplyMail.Save()` draft contract.

---

## Track B: Runner Execution, Concurrency, Process & HTTP Corrections

### 6. 100% Python Removal
- **Strategy**:
  - Audit all Runner tests and purge 100% of Python scripts/references (`python`, `python3`, `python.exe`).
  - Replace with native Rust mechanisms (`std::thread`, tokio tasks, `Barrier`, channels, atomics, Rust worker binaries).

### 7. Production-Path Period Execution Behavioral Tests
- **File**: `src/runner/engine/pipeline.rs`
- **Strategy**:
  - Test sequential period execution through `execute_step`/`execute_pipeline` proving non-overlapping, strictly ordered completion.
  - Test concurrent period execution through `execute_step`/`execute_pipeline` proving observed simultaneous activity using `tokio::sync::Barrier`.
  - Test concurrent error propagation ensuring task failure status is set and propagated correctly.

### 8. Real Duplicate Admission Race Test
- **File**: `src/runner/engine/dispatcher/lifecycle.rs`
- **Strategy**:
  - Spawn two concurrent callers gated by a `tokio::sync::Barrier` calling `run_task_by_id` against `ExecutionManager` for the same task ID.
  - Assert exactly 1 instance is admitted and duplicate launch is rejected atomically.

### 9. Process Tree Timeout Termination Test
- **File**: `src/runner/engine/process.rs`
- **Strategy**:
  - Implement a behavioral process tree test spawning a parent process that launches a child process tree.
  - Trigger timeout in `run_process` and verify `terminate_process_tree` (`taskkill /F /T /PID` on Windows, `pkill -P` on Unix) terminates both parent and child processes.

### 10. Bounded Process Output Test (>10 MiB Pipe)
- **File**: `src/runner/engine/process.rs`
- **Strategy**:
  - Implement a process test spawning a process writing >10 MiB to stdout/stderr pipes.
  - Verify `read_bounded` caps captured memory strictly at `MAX_OUTPUT_BYTES = 10MB` per stream without memory explosion or panics, and process cleanup completes cleanly.

### 11. Complete HTTP Server Hardening Test Suite
- **File**: `src/runner/gui/mod.rs`
- **Strategy**:
  - Test exact 64KB header limit and 2MB body limit boundaries (+1 byte returns 413 Payload Too Large).
  - Test fragmented CRLF (`\r\n\r\n`) and LF (`\n\n`) headers and bodies over multiple TCP read chunks.
  - Test 10-second socket read timeout returning HTTP 408 Request Timeout and closing connection.
  - Test chunked transfer encoding rejection (`400 Bad Request`).
  - Test GET mutation rejection (`405 Method Not Allowed`) across all state-changing endpoints.
  - Test non-loopback IP binding validation (`0.0.0.0` rejected).
  - Test reason phrases for all status codes.

### 12. Async Handler Blocking I/O Audit
- **Files**: `src/runner/gui/handlers.rs`, `routes.rs`
- **Strategy**:
  - Audit all async HTTP handlers (`/api/apps/manifest`, etc.) and offload external process calls using Tokio async process (`tokio::process::Command::new(...).output().await`).

### 13. Documentation Restoration
- **Files**: `md/TASKER.md`, `md/RUNNER.md`
- **Strategy**:
  - Restore all previously removed operational documentation and update them with detailed descriptions of ScriptManager versioning/locking, HTTP security model, process tree timeout termination, and CLI parameterization contracts.

---

## Verification Suite
- `cargo fmt --all -- --check`
- `cargo test --all-targets --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
