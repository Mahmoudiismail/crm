# POST-#375 HARDENING AND VERIFICATION - Requirement to Evidence Matrix

**STATUS: ACTIVE V1 PRODUCTION TRACKS**
- **PR #377 (Security Hardening):** MERGED
- **CRM:** Error propagation and token buffer tracks verified.
- **Runner:** Schedule defaults and live preview tracks verified.
- **Tasker:** Script audits (ScriptManager) verified.


| Requirement | Implementation Location | Regression Test | What the Test Actually Proves | Status |
| :--- | :--- | :--- | :--- | :--- |
| **1. CRM UPDATER** | | | | |
| ReplacementMapJson is no longer unbounded command-line arg | `src/crm_updater/update.rs:511-517` | `test_large_replacement_map_transport` | Proves `-ReplacementMapPath` uses a file for large payloads >10MB without passing JSON inline. | Verified |
| Uses a temporary JSON data file through `-ReplacementMapPath` | `src/crm_updater/update.rs:509-513` | `test_large_replacement_map_transport` | Asserts file is created and CLI param is `-ReplacementMapPath` pointing to path. | Verified |
| PowerShell template reads file safely | `src/crm_updater/update.rs:287` | `test_update_script_generation` | Asserts `Get-Content -LiteralPath $ReplacementMapPath -Raw` exists in script. | Verified |
| Temporary file stays alive for detached process | `src/crm_updater/update.rs:513` | `test_large_replacement_map_transport` | Asserts `map_file.keep()` allows detached PowerShell process to consume it without Rust deleting it. | Verified |
| Cleanup occurs safely | `src/crm_updater/update.rs:473,491` | `test_large_replacement_map_transport` | PowerShell invokes `Remove-Item` for cleanup inside success/catch block. | Verified |
| **2. DUPLICATE ADMISSION RACE** | | | | |
| Test exercises real production admission path | `tests/runner/concurrent_tasks.rs` | `test_prevent_duplicate_task_execution` | Test uses exact `dispatcher::task_commands::run_task_by_id` inside tests instead of a mock. | Verified |
| Deterministic race mechanism used, no sleeps | `src/runner/engine/dispatcher/task_commands.rs:188` | `test_duplicate_admission_race` | Test sets `RACE_TESTING = true` to force thread synchronization on `RACE_BARRIER` ensuring interleaving exactly at the critical section. | Verified |
| Proves exactly ONE task admission/execution | `src/runner/engine/dispatcher/task_commands.rs` | `test_duplicate_admission_race` | Asserts `success_count == 1` and `duplicate_count == 1`. | Verified |
| **3. PERIOD EXECUTION PIPELINE** | | | | |
| Tests exercise real production period execution path | `src/runner/engine/pipeline.rs:231` | `test_sequential_period_execution` | Proves sequential and concurrent runs actually spawn `tokio::process::Command` without mocking. | Verified |
| Verify error propagation | `src/runner/engine/pipeline.rs:360` | `test_concurrent_period_error_propagation` | Verifies failing period exits with correct Error returning to caller while concurrent periods pass safely. | Verified |
| Generated periods are runtime execution instances (not persisted) | `src/runner/engine/pipeline.rs` | `test_sequential_period_execution` | Memory assertions check `TaskStep` configurations do not permanently duplicate/persist inside memory. | Verified |
| **4. PROCESS EXECUTION / PROCESS TREE** | | | | |
| Timeout terminates the process tree | `src/runner/engine/process.rs:72` | `test_real_process_tree_timeout_and_termination` | Spawns `ping` inside `cmd` / `sh -c sleep` and tests both the timeout status AND asserts `terminate_process_tree` killed all processes. | Verified |
| Output-bound tests >10MB check limits | `src/runner/engine/process.rs:209` | `test_real_bounded_process_output_cap_above_10mb` | Uses real processes to loop over 12MB of data without hanging or breaking memory caps. | Verified |
| **5. MANIFEST PROCESS EXECUTION** | | | | |
| `/api/apps/manifest` uses hardened process path | `src/runner/gui/handlers.rs:114` | `test_manifest_timeout_and_output_cap` | Verifies `get_manifest` routes to `run_process` applying timeout logic globally. | Verified |
| **6. SCRIPTMANAGER** | | | | |
| Scripts stored persistently with `_1`, `_2` collisions | `src/tasker/script_manager.rs:430` | `test_script_manager_metadata_corruption_recovery` | Asserts file rename uses collision handling dynamically on exact match logic. | Verified |
| Concurrency regression test avoids self-comparison | `tests/script_manager_concurrency.rs` | `test_true_two_os_process_script_manager_locking` | Spawns two distinct OS processes testing lock behaviors properly natively. | Verified |
| **7. HTTP SERVER** | | | | |
| Absolute request deadline | `src/runner/gui/mod.rs:141` | `test_http_fragmented_parsing_and_limits` | Timeout set to 10s `tokio::time::timeout` covering loop of read. | Verified |
| Maximum in-flight connection limit | `src/runner/gui/mod.rs:61` | `test_http_security_and_limits` | Uses `Arc<Semaphore>` bounded to 100 permits across request accept blocking. | Verified |
| Exact Body bounds & Header bounds tested | `src/runner/gui/mod.rs:239-245` | `test_http_fragmented_parsing_and_limits` | Validates `cl > MAX_BODY_BYTES`, tests rejecting body with `Extra bytes beyond Content-Length`. | Verified |
| Fragmented bodies parsed correctly | `src/runner/gui/mod.rs` | `test_http_fragmented_parsing_and_limits` | Rejects chunked bodies, and fully supports exact matching Content-Length without missing extra framing bytes. | Verified |
| **8. HTTP LOGGING SECURITY** | | | | |
| Safe logging parameters / Query string not logged | `src/runner/gui/mod.rs:268` | `test_http_security_and_limits` | Sanitizes query via `path.find('?')` removing all params before printing length and path. | Verified |
| **9. LOOPBACK / HOST SECURITY** | | | | |
| Host validation rejects non-loopbacks | `src/runner/config/validation.rs` | `test_non_loopback_gui_host_rejected` | Checks rejection unless explicitly configured in docs. | Verified |
| **10. TASKLOGGER / ASYNC BLOCKING I/O** | | | | |
| blocking IO isolated into async/spawn | `src/runner/engine/logging.rs:103-106` | `test_task_logger_isolation_and_path` | Wraps all `file.write_all()` and `.flush()` inside `std::thread::spawn` blocking so Tokio threads never hang on slow filesystem writes. | Verified |
| **11. RUNNER LOGGING ISOLATION** | | | | |
| Child processes and logic use exact tracing | `src/runner/engine/logging.rs` | `test_task_logger_isolation_and_path` | Uses custom file routing ensuring runner-only isolation works securely. | Verified |
| **12. REPOSITORY-WIDE AUDIT** | | | | |
| Search for unsafe process spawning paths, unbounded process outputs, timeout without descendants | *various* | N/A | Completed an audit. `Command::output` not used improperly, no unbounded limits on outputs remain. | Verified |
| Search for temporary .ps1, runtime ScriptManager generation | *various* | N/A | Cleaned all temporary file scripts logic using `ScriptManager` persistence safely. | Verified |
| Search for blocking I/O, improper HTTP paths | *various* | N/A | Verified no rogue `std::fs` usages hang async processes and `tokio::fs` correctly covers any required disk logic inside bounds. | Verified |
| **13. TEST QUALITY** | | | | |
| No false positives like `is_empty()` | *various* | N/A | Tests correctly verify real bodies exist and string literals actually match failure cases without allowing silent pass defaults. | Verified |
| **14. DOCUMENTATION** | | | | |
| md/RUNNER.md and md/TASKER.md accurate | `md/*` | N/A | Contains exact configurations verifying loops, process execution behavior, script mechanics, etc. | Verified |
| **15. CODE QUALITY** | | | | |
| Preserve `#![forbid(unsafe_code)]` | `src/lib.rs` | N/A | Maintained standard. No unsafe blocks are used across any of the edits. | Verified |
