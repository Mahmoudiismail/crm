# P0-A Tasker PowerShell + ScriptManager Remediation Plan (Revised)

## Executive Summary
This plan details the comprehensive remediation of all PowerShell script generation, execution, path safety, OS-level locking, and `ScriptManager` versioning across the entire codebase.

---

## 1. Comprehensive Repository-Wide Audit & Classification

Every production PowerShell execution path found in the repository is classified below:

| File / Location | Script / Usage | Current Mechanism | Classification | Planned Remediation |
|---|---|---|---|---|
| `src/tasker/crm_open_sohail/mod.rs` | `reply_email.ps1` | Malformed/broken template + CLI args | **A** (Refactor to Static) | Reconstruct static template with `[string]$SenderAccount`, `[string]$SubjectPrefix`, `[string]$Subject`, `[string]$HtmlBody` and fix variable bindings. |
| `src/tasker/crm_open_sohail/powershell.rs` | `slicer_extract.ps1` | Static template + CLI args | **B** (Already Static) | Verify parameter contract; no structural changes required. |
| `src/tasker/department_split.rs` | `department_split.ps1` | Static template + CLI args | **B** (Already Static) | Verify parameter contract; no structural changes required. |
| `src/tasker/dashboard_updater.rs` | `dashboard_update.ps1` & `dashboard_email.ps1` | Static template + CLI args | **B** (Already Static) | Verify parameter contract; no structural changes required. |
| `src/tasker/opd_task/powershell_email.rs` | `opd_analysis_email.ps1` | Static template + CLI args | **B** (Already Static) | Verify parameter contract; no structural changes required. |
| `src/tasker/email/outlook.rs` & `src/tasker/email/client.rs` | `send_email.ps1` | Dynamic string interpolation in `client.rs` passed to `outlook::run_powershell` | **A** (Refactor to Static) | Refactor `send_email.ps1` to static template with parameters `[string]$To`, `[string]$Cc`, `[string]$Subject`, `[string]$HtmlBody`, `[string]$AttachmentPath`, `[string]$LeadsPath`, `[string]$DisplayOrSend`. Pass values via `execute_script_with_args`. |
| `src/crm_updater/logs.rs` | `send_logs_email` | Dynamic string interpolation via `format!` calling `run_powershell` | **A** (Refactor to Static) | Refactor to static template `send_logs.ps1` accepting parameters `[string]$Recipient`, `[string]$AttachmentsCsv`. Pass attachments cleanly without inline script formatting. |
| `src/crm_updater/update.rs` | `download_update_zip_from_drafts` | Dynamic PS script string written to temp `.ps1` | **A** (Refactor to Static) | Refactor to static template `scan_drafts.ps1` accepting parameter `[string]$DownloadsDir`. Pass directory path via CLI argument. |
| `src/crm_updater/update.rs` | `unblock_file` | `powershell -Command "param([string]$Path) Unblock-File -LiteralPath $Path"` | **B** (Already Static) | Parameter is passed positionally via `.arg(path)` to script block; no script interpolation exists. |
| `src/crm_updater/update.rs` | `generate_update_script` & `execute_detached_powershell` | Dynamic PS string constructed in Rust for detached process | **A** (Refactor to Static) | Refactor detached updater script into a static parameterized script `detached_update.ps1` accepting `[string]$ConfigJsonPath`. Rust writes structured runtime JSON config to a temp file, passing `-ConfigJsonPath` as a CLI argument to the static script. |

---

## 2. PowerShell Argument Safety & Injection Tests

### Argument Passing Mechanism
For all PowerShell executions, runtime parameters are supplied strictly via `std::process::Command` CLI arguments:
```rust
cmd.arg("-ParameterName").arg(runtime_value_string);
```
On Windows, `std::process::Command` passes parameters to executable parameter blocks without invoking `cmd.exe` shell parsing, guaranteeing that parameter values cannot be interpreted as PowerShell code statements or metacharacters.

### Hostile Metacharacter & Injection Tests
Add native Rust injection safety tests passing hostile test strings containing:
`'`, `"`, `$`, `$()`, `` ` ``, `;`, `|`, `&`, `\n`, `\r`

Test suite will verify that:
1. Canonical template content and fingerprint remain completely unchanged.
2. Argument strings are received verbatim inside PowerShell parameter variables without executing code payloads.
3. Hostile inputs cannot modify script logic or structure.
4. No timestamped script versions are generated as a result of hostile argument inputs.

---

## 3. ScriptManager Architecture, Fingerprinting & Atomic Version Selection

### OS-Level Lock Coverage
The `fs2` exclusive lock on `<task_dir>/.task.lock` covers the entire critical section:
1. Acquire exclusive OS lock (`FileExt::lock_exclusive`).
2. Read `.metadata.json` (or recover corrupted metadata).
3. Compute SHA-256 fingerprint of canonical static script template.
4. Compare generator fingerprint against active entry in metadata.
5. If fingerprint matches and active script exists, reuse active script.
6. If fingerprint differs or script is missing:
   - Perform atomic version selection (`<stem>.ps1`, `<stem>_YYYY-MM-DD_HH-MM-SS.ps1`, `<stem>_YYYY-MM-DD_HH-MM-SS_1.ps1`, etc.).
   - Write new script content to `.tmp` file in task directory and `fs::rename` to target script file.
   - Update metadata JSON in memory.
   - Write updated metadata to `.metadata.json.tmp` and `fs::rename` to `.metadata.json`.
7. Release OS lock (`FileExt::unlock`).

*Stale-lock policy*: Time-based stale lock deletion is strictly forbidden and removed. A lock is held solely for the duration of the critical section and released upon scope drop or explicit unlock.

### Deterministic Version Collision Selection
Version selection logic handles timestamp collisions deterministically:
- With a fixed/mocked clock producing timestamp `2026-03-30_12-00-00`:
  - If `script.ps1` exists -> target is `script_2026-03-30_12-00-00.ps1`.
  - If `script_2026-03-30_12-00-00.ps1` exists -> target is `script_2026-03-30_12-00-00_1.ps1`.
  - If `script_2026-03-30_12-00-00_1.ps1` exists -> target is `script_2026-03-30_12-00-00_2.ps1`.

### Path Safety Validation
Strict path safety validation is applied independently to both logical script names requested by callers and `active_script` values read from metadata files:
- Reject absolute paths (`C:\...`, `/...`).
- Reject parent directory traversal (`..`, `../`, `..\`).
- Reject path separators (`/`, `\`) and drive letters (`:`).
- Verify resolved target script resides strictly within `<exe_dir>/scripts/<Task Folder>/`.

### Corrupted Metadata Recovery
When `.metadata.json` contains malformed JSON or invalid/malicious `active_script` paths:
- Log warning/error.
- Rename corrupted metadata file to `.metadata.json.corrupted_<timestamp>`.
- Reinitialize clean `TaskMetadata` without deleting existing `.ps1` script files.

---

## 4. Behavior Preservation & Detailed Component Remediation

### A. `reply_email.ps1` (`src/tasker/crm_open_sohail/mod.rs`)
- **Parameter Interface**:
  ```powershell
  param(
      [string]$SenderAccount,
      [string]$SubjectPrefix,
      [string]$Subject,
      [string]$HtmlBody
  )
  ```
- **Logic Restoration**:
  - Search Inbox and Sent Items for messages matching `$SubjectPrefix` and `$SenderAccount` (using `GetExchangeUser().PrimarySmtpAddress` and MAPI property `0x39FE001E` fallback).
  - Select most recent matching message.
  - Call `.ReplyAll()`.
  - Set `.Subject = $Subject`.
  - Prepend `$HtmlBody` to `.HTMLBody`.
  - Call `.Save()` to create a draft in Outlook.
  - Wrap in `try/catch` block and explicitly exit non-zero on failure.
- **Verification Tests**:
  - Test parameter signature matching between Rust caller and PowerShell template.
  - Test exact variable bindings and error handling.

### B. `send_email.ps1` (`src/tasker/email/outlook.rs` & `src/tasker/email/client.rs`)
- **Parameter Interface**:
  ```powershell
  param(
      [string]$To,
      [string]$Cc,
      [string]$Subject,
      [string]$HtmlBody,
      [string]$AttachmentPath,
      [string]$LeadsPath,
      [string]$DisplayOrSend
  )
  ```
- **Logic**:
  - Initialize Outlook COM, set properties cleanly without string concatenation.
  - Handle optional attachment additions safely.
  - Execute `$Mail.Send()` or `$Mail.Display()` based on `$DisplayOrSend`.
- **Verification Tests**:
  - Test sending and display modes with mocked or parameterized calls.

### C. `send_logs.ps1` (`src/crm_updater/logs.rs`)
- **Parameter Interface**:
  ```powershell
  param(
      [string]$Recipient,
      [string]$AttachmentsCsv
  )
  ```
- **Logic**:
  - Parse comma-separated `$AttachmentsCsv`, attach each valid file in a `try/catch` block, and send email.

### D. `scan_drafts.ps1` (`src/crm_updater/update.rs`)
- **Parameter Interface**:
  ```powershell
  param(
      [string]$DownloadsDir
  )
  ```
- **Logic**:
  - Scan Outlook Drafts folder for update ZIP matching `^crm_tool_.*\.zip$`, save attachment to `$DownloadsDir`, output `FOUND:<path>` or `NOT_FOUND`.

### E. `detached_update.ps1` (`src/crm_updater/update.rs`)
- **Parameter Interface**:
  ```powershell
  param(
      [string]$ConfigJsonPath
  )
  ```
- **Logic**:
  - Read JSON configuration containing parent PID, log path, downloads dir, file replacement map, and autostart flags.
  - Execute process termination, SHA-256 verification, file replacement, autostart, and cleanup.

---

## 5. True Two-Process Integration Test

Add a native Rust integration test (`tests/script_manager_concurrency.rs`):
1. Spawn two independent OS processes using `std::process::Command::new(std::env::current_exe())` with a hidden test flag (`--test-scriptmanager-process`).
2. Both child processes will concurrently invoke `ScriptManager::get_or_create_script` on the same temporary task root directory across multiple iterations.
3. Deterministic synchronization: Processes synchronize start via a shared barrier lock file or pipe before entering the critical section.
4. Assertions:
   - Valid final `.metadata.json` (valid JSON, parseable).
   - `active_script` exists on disk.
   - Every script version referenced in metadata exists on disk.
   - No lost updates or corrupt state.
   - No duplicate version filenames allocated.

---

## 6. Persistence Location & No TEMP Persistence

- Confirm all Tasker production scripts reside in `<exe_dir>/scripts/<Task Folder>/`.
- Validate that no production execution path creates persistent `.ps1` files under `%TEMP%`.

---

## 7. Plan Steps & Execution Order (Post-Authorization)

1. **Step 1**: Fix `reply_email.ps1` in `src/tasker/crm_open_sohail/mod.rs` with full parameter interface and behavioral tests.
2. **Step 2**: Refactor `send_email.ps1` in `src/tasker/email/` and `send_logs_email` in `src/crm_updater/logs.rs` to static parameterized scripts.
3. **Step 3**: Refactor `src/crm_updater/update.rs` (`scan_drafts.ps1`, `detached_update.ps1`) to static parameterized scripts.
4. **Step 4**: Enhance `ScriptManager` with path safety validation, corrupted metadata recovery, and atomic `.tmp` file operations.
5. **Step 5**: Add injection safety, fingerprinting, manual edit preservation, and deterministic timestamp collision unit tests.
6. **Step 6**: Implement true two-OS-process integration test for `ScriptManager` lock concurrency.
7. **Step 7**: Perform repository-wide verification, pre-commit checks (`cargo fmt`, `cargo test`, `cargo clippy`), and verify complete checklist.
