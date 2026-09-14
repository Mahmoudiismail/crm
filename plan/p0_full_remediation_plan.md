# P0 Full Critical Remediation Plan (Track A + Track B)

## Overview
This plan establishes the execution, testing, review, and verification strategy for the P0 Remediation task across both Track A (PowerShell & ScriptManager) and Track B (Runner Execution, Concurrency, Process & HTTP).

---

## Track A: PowerShell + ScriptManager

### A1 & A2. Repository-Wide PowerShell Audit & CRM Updater Remediation
- **Affected File**: `src/crm_updater/update.rs`
- **Scope**:
  - Remove all dynamic string building (`format!`, `push_str`, string concatenation) that interpolates runtime values into PowerShell source code across `download_update_zip_from_drafts()`, `generate_update_script()`, `execute_detached_powershell()`, `unblock_file()`, and all associated helpers.
  - Define static canonical PowerShell script templates parameterized using `param(...)` blocks.
  - Supply runtime values (e.g., `-DownloadsDir`, `-ParentPid`, `-TargetPaths`, `-LogPath`) strictly via CLI arguments using parameter flags (`cmd.arg("-ParamName").arg(param_value)`).
  - Preserve all existing updater semantics: PID termination wait, SHA-256 hash checking, download/extraction/replacement, autostart behavior, exit code 1 on failure, detached process creation, and file cleanup.
  - Perform a post-refactor repository-wide audit for any remaining unparameterized PowerShell generation.

### A3. Reply Email PowerShell
- **Affected File**: `src/tasker/crm_open_sohail/mod.rs`
- **Scope**:
  - Restore `ps_email_template` to a complete, valid static parameterized PowerShell implementation. Fix all syntax errors, missing variable bindings, and truncated variables (`$Inbox`, `$SentFolder`, `Find-OriginalMessage`, `$matches`, `$script:OriginalMail`, `$TargetMail.ReplyAll()`, `$ReplyMail.HTMLBody`, `$ReplyMail.Save()`).
  - Pass parameters via CLI (`-SenderAccount`, `-SubjectPrefix`, `-Subject`, `-HtmlBody`).
  - Update unit tests in `mod.rs` to verify the correct contract, asserting `$ReplyMail.Save()` is called and draft status is preserved without corrupted test assertion strings.

### A4. PowerShell Injection Safety
- **Affected Files**: `src/tasker/script_manager.rs`, `tests/`
- **Scope**:
  - Ensure all runtime-controlled values are passed via parameter CLI arguments rather than string interpolation.
  - Refactor `test_argument_metacharacters_injection_safety_and_fingerprint` to use a fixed valid parameter name (`HostileVal`) while passing hostile parameter values containing single quotes, double quotes, semicolons, `$`, backticks, pipes, ampersands, CR/LF, parens, subexpressions, and redirection characters.
  - Perform real PowerShell execution tests where available, and structural parameter assertions where PowerShell is unavailable.

### A5–A12. ScriptManager Core Correctness, Fingerprinting & Locking
- **Affected File**: `src/tasker/script_manager.rs`
- **Scope**:
  - Compute script fingerprints exclusively from canonical static script templates. Runtime arguments must NOT alter the generator hash or create new versions.
  - Reject path traversal, absolute paths, and unsafe script names.
  - Handle corrupted metadata safely: backup corrupted JSON to `.metadata.json.corrupted_<timestamp>`, log errors, and initialize valid replacement metadata.
  - Implement atomic file/metadata writes using temporary files (`.tmp`) flushed and synced before renaming.
  - Protect the complete critical section (`read metadata -> recovery -> fingerprint comparison -> active script validation -> version selection -> script creation -> metadata update`) using OS-level cross-process locking (`fs2::FileExt::lock_exclusive` on `.task.lock`).
  - Preserve active user-edited scripts when the generator hash is unchanged.
  - Ensure deterministic timestamp collision resolution (`_1`, `_2`, etc.) without random sleeps.
  - Implement a genuine multi-process cross-process concurrency test spawning two independent OS processes contending for script allocations under deterministic synchronization (barrier/IPC).

### A13–A14. Error Handling & No Temporary Tasker PowerShell
- **Scope**: Remove unnecessary `unwrap()`/`expect()` from domain logic, including PowerShell output parsing. Verify no Tasker `.ps1` files are created under `TEMP`.

---

## Track B: Runner Execution / Concurrency / Process / HTTP

### B1. 100% Python Removal from Runner Tests
- **Scope**: Search all Runner tests for `python` or `python3` and replace them with native Rust mechanisms (`std::thread`, `tokio::sync::Barrier`, channels, atomics, or child Rust test binaries).

### B2–B5. Period Execution, Error Propagation & Admission Race
- **Affected Files**: `src/runner/engine/pipeline.rs`, `src/runner/engine/dispatcher.rs`
- **Scope**:
  - Add behavioral tests for sequential period execution proving non-overlapping, ordered completion.
  - Add behavioral tests for concurrent period execution proving actual overlapping execution using Rust synchronization barriers/atomics.
  - Add behavioral tests for concurrent error propagation ensuring task failure status is properly set and propagated.
  - Implement a real race test for duplicate task admission where two concurrent callers attempt to launch the same task ID on `ExecutionManager` simultaneously under deterministic barrier gating, proving exactly one is admitted and the second is rejected.

### B6–B9. Preview Parity, Date Resolution, Working Hours & Post-Run
- **Affected Files**: `src/runner/config/schedule.rs`, `src/runner/config/periodization.rs`, `src/runner/gui/routes.rs`
- **Scope**:
  - Ensure preview generation `/api/tasks/preview` calls `generate_upcoming_executions_for_app` on the backend date engine.
  - Ensure relative date expressions evaluate sequentially (e.g. "next Saturday" for both start and end date resolves to the exact same upcoming Saturday).
  - Ensure non-working days filter both actual execution and preview occurrences (skipped occurrences omitted from preview). Limit preview to max 10 upcoming occurrences.
  - Ensure post-run step periodization applies without mutating persisted `TaskStep` configurations.

### B10–B11. Application Locking & Lifecycle Status
- **Affected Files**: `src/runner/engine/app_lock.rs`, `src/runner/engine/pipeline.rs`
- **Scope**:
  - Verify step-scoped app locking: deduplicate app IDs, sort deterministically to avoid deadlocks, acquire exclusive lock for `allow_concurrent_tasks=false` apps, hold permits for step duration, and auto-release on normal completion, error, or cancellation.
  - Ensure tasks waiting for app locks remain in `running_task_ids` and report `waiting_for_app` status.

### B12–B13. Process Timeout & Bounded Output
- **Affected File**: `src/runner/engine/process.rs`
- **Scope**:
  - Strengthen timeout process tree termination (`taskkill /F /T /PID` on Windows, `pkill -P` on Unix) and verify process descendants are terminated.
  - Test bounded memory output beyond 10MB per stream, proving memory is capped at 10MB without panics and process cleanup completes.

### B14–B22. HTTP Server Hardening
- **Affected File**: `src/runner/gui/mod.rs`
- **Scope**:
  - Fix parser line-ending consistency between `\r\n\r\n` (CRLF) and `\n\n` (LF) across header detection, Content-Length calculation, and body extraction.
  - Enforce MAX_HEADER_BYTES (64KB) and MAX_BODY_BYTES (2MB) exact boundary limits and 1-byte-over behavior.
  - Implement 10-second TCP read timeout returning HTTP 408 Request Timeout and closing connection.
  - Reject chunked transfer encoding with HTTP 400 Bad Request.
  - Reject GET requests on state-changing endpoints with HTTP 405 Method Not Allowed.
  - Enforce non-loopback binding validation (general non-loopback IPs rejected).
  - Ensure correct status reason phrases.

### B23. Blocking I/O Audit in Async Handlers
- **Scope**: Audit `/api/apps/manifest` and all async HTTP handlers to offload any blocking process calls using `tokio::process::Command` or `tokio::task::spawn_blocking`.

---

## Cross-Cutting Audits & Verification

1. **Repository-Wide Audits**: Repeat audits for PowerShell string interpolation, Python in tests within scope, blocking I/O, and `unwrap()`/`expect()` usage.
2. **Documentation**: Update `md/TASKER.md`, `md/RUNNER.md`, and relevant docs under `md/`.
3. **Mandatory Reviews**: Perform PRE-TEST Internal Code Review and FINAL Internal Code Review around testing.
4. **Verification Suite**:
   - `cargo fmt --all -- --check`
   - `cargo test --all-targets`
   - `cargo clippy --all-targets --all-features -- -D warnings`
