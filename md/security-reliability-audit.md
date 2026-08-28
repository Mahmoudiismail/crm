# Session 4 — Full Security & Reliability Audit

## 1. Executive Summary
This read-only audit assessed the CRM Tool architecture, focusing on the Runner execution pipeline, configuration ingestion, Custom TCP HTTP server, CSV processing, and process life-cycle. The review confirmed the codebase correctly limits external command execution by decoupling arguments from execution logic `(tokio::process::Command::arg)`. However, several P1 and P2 reliability/security issues were uncovered regarding unbounded reads in the custom TCP server, blocking `std::fs` calls within Tokio async contexts (starvation risk), PowerShell string interpolation risks, and the persistence of `.flexible(true)` within the `tasker` CSV configuration against explicit rules.

## 2. Overall Security Risk
**Medium.** The application boundary is relatively tight because the Runner is designed to run locally (binding to `127.0.0.1` by default). However, the lack of authentication on the GUI API routes and the use of unescaped string interpolation in PowerShell commands present significant lateral movement and command injection risks if an attacker gains access to the local network or configuration files.

## 3. Overall Reliability Risk
**High.** The most critical reliability risks stem from the custom TCP HTTP server implementation, which lacks bounds checking and can be trivially memory-exhausted. Furthermore, synchronous file I/O operations (like `walkdir` and `std::fs::remove_file`) inside async Tokio workers present a starvation risk to the scheduler.

## 4. P0 Findings
None identified.

## 5. P1 Findings

### [P1] Unbounded Request Parsing in Custom HTTP Server
**Confidence:** High
**Location:** `src/runner/gui/mod.rs:read_http_request`
**Evidence:** The TCP listener dynamically resizes a `vec![0u8; 8192]` infinitely (`buf.resize(buf.len() * 2, 0)`) while waiting for a `\r\n\r\n` boundary or full `Content-Length`.
**Trigger:** An unauthenticated attacker sending an infinitely long HTTP header line without `\r\n\r\n` or a massive payload.
**Impact:** Denial of Service (DoS). The vector will grow until Out-Of-Memory (OOM), killing the `runner` daemon.
**Affected component:** Runner GUI
**Recommended remediation:** Enforce a hard cap (e.g., 1MB) on `buf.len()` during the read loop and immediately terminate the socket if exceeded.
**Regression test:** A test opening a TCP connection and sending 5MB of garbage data without `\r\n\r\n` to ensure it drops the connection without crashing.

### [P1] `std::fs::remove_file` and `walkdir` block Tokio Worker Threads
**Confidence:** High
**Location:** `src/runner/engine/logging.rs:cleanup_old_logs`
**Evidence:** The daily scheduler spawns `cleanup_old_logs` onto a standard `tokio::spawn` worker. Inside, it uses synchronous `walkdir::WalkDir` and `std::fs::remove_file` inside a loop over log files.
**Trigger:** Triggered automatically every 24 hours via cron schedule.
**Impact:** If the `logs/` directory contains thousands of files, the synchronous iteration and deletion will lock a Tokio worker thread for hundreds of milliseconds or seconds, stalling async scheduling and execution pipelines.
**Affected component:** Runner Dispatcher
**Recommended remediation:** Wrap `cleanup_old_logs` in `tokio::task::spawn_blocking` or use `tokio::fs::remove_file` and `tokio::fs::read_dir`.
**Regression test:** `test_log_cleanup_does_not_block_executor` asserting async yields occur during massive directory sweeps.

### [P1] `csv::ReaderBuilder` continues to bypass strict validation (`.flexible(true)`)
**Confidence:** High
**Location:** `src/tasker/csv_task.rs:generate_csv`
**Evidence:** The CSV parser is explicitly configured with `csv::ReaderBuilder::new().flexible(true)`.
**Trigger:** Parsing dynamically grouped reports via `tasker` background jobs.
**Impact:** Malformed CSV datasets with missing/extra columns will be silently accepted, violating the strict validation mandated in `AGENTS.md`. Downstream index-based lookups may misalign or panic.
**Affected component:** Tasker
**Recommended remediation:** Change `.flexible(true)` to `.flexible(false)` and allow `from_reader` to propagate validation errors.
**Regression test:** A unit test verifying `generate_csv` strictly rejects jagged CSV inputs.

## 6. P2 Findings

### [P2] Path Traversal bypassing `resolve_relative_to_exe_dir`
**Confidence:** High
**Location:** `src/runner/engine/validation.rs:resolve_relative_to_exe_dir`
**Evidence:** The helper checks `if p.is_absolute() { return p; }` and immediately returns absolute paths without enforcing a jail.
**Trigger:** A user or script configures an `ExternalApp` config or a `CsvAnalysisConfig` download path to `/etc/shadow` or `C:\Windows\System32`.
**Impact:** While it prevents `../../` escapes relative to the exe, it completely permits accessing absolute root locations on disk, allowing configuration to arbitrarily manipulate the host filesystem.
**Affected component:** Runner, Tasker
**Recommended remediation:** Check if absolute paths are allowed globally, or strictly sandbox configurations by validating the canonicalized path is a sub-directory of `current_exe()`.
**Regression test:** Assert that `resolve_relative_to_exe_dir("/etc/passwd")` fails or strips the absolute boundary if strict jailing is desired.

### [P2] PowerShell Script Execution via string interpolation
**Confidence:** Medium
**Location:** `src/tasker/department_split.rs:run`
**Evidence:** Extensive use of `format!(r#" $outDir = '{out_dir}' "#)` to dynamically generate `.ps1` files.
**Trigger:** A user crafts a filename or CSV config mapping containing a single quote `'` (e.g., `' ; Invoke-WebRequest... ; #`).
**Impact:** PowerShell command injection. Dynamic inputs or paths flowing into `ps_script` break the execution trust boundary.
**Affected component:** Tasker
**Recommended remediation:** Pass variables to PowerShell scripts via explicit script parameters (`-File script.ps1 -OutDir ...`) instead of string interpolation.
**Regression test:** Test executing `department_split` with a dashboard path containing `' ; Exit 1 ; #` to ensure it safely resolves as a literal string.

## 7. P3 Findings

### [P3] Headless Chrome 60-Second Sleep on Failure
**Confidence:** High
**Location:** `src/yasweb/browser/mod.rs:run_browser_tab`
**Evidence:** If `login::execute_login` fails and `config.keep_open` is true, the thread executes `std::thread::sleep(Duration::from_secs(60))`.
**Trigger:** A login failure or UI timeout while `--keep-open` is active.
**Impact:** Since `run_browser_tab` is inside `tokio::task::spawn_blocking`, it won't starve the async executor, but it needlessly consumes a dedicated OS blocking thread for 60 seconds.
**Affected component:** YasWeb
**Recommended remediation:** Remove the blocking sleep and rely on native Playwright/DevTools tracing or external debugger attachments.
**Regression test:** Assert that YasWeb execution returns an error immediately upon login failure without blocking for 60 seconds.

### [P3] Unauthenticated Task Management
**Confidence:** High
**Location:** `src/runner/gui/routes.rs:route_request`
**Evidence:** The Runner HTTP API (`/create`, `/run`, `/update`) performs zero authentication checks.
**Trigger:** A user navigates to `/run/task_id` or posts to `/create`.
**Impact:** Because tasks can be `ShellCommand` types, an attacker on the same network interface could achieve RCE via the `/create` API. Mitigated heavily by default `127.0.0.1` bindings.
**Affected component:** Runner GUI
**Recommended remediation:** Introduce basic API token authentication or HTTP Basic Auth for GUI routes.
**Regression test:** A test hitting `/create` without an Authorization header ensuring it returns 401 Unauthorized.

### [P3] Secret Exposure in `Debug` Prints
**Confidence:** Medium
**Location:** `src/bin/crm.rs:main`
**Evidence:** The application prints `info!("Loaded config: {:#?}", config);`. The `AppConfig` struct explicitly implements a custom `Debug` that redacts the `password` field via `***REDACTED***`.
**Trigger:** Standard application startup logs `AppConfig`.
**Impact:** Mitigated by the custom `Debug` implementation. However, any new structs embedding `password: String` directly risk exposure.
**Affected component:** CRM, YasWeb
**Recommended remediation:** Use `secrecy::SecretString` for password fields to enforce zeroize and redaction natively.
**Regression test:** Assert that `format!("{:?}", config)` does not contain the literal password string.

## 8. HTTP Security
The custom TCP implementation (`src/runner/gui/mod.rs`) manually reads streams into a resizing vector until it matches `Content-Length`. It completely ignores standard HTTP protections like `Transfer-Encoding`, connection timeouts, and maximum header limits, leaving the `runner` highly exposed to Slowloris and OOM DoS attacks.

## 9. Authentication & Authorization
The Runner application completely lacks authentication. The GUI routes (`/create`, `/run`, `/update`) in `src/runner/gui/routes.rs` accept unauthenticated parameters directly into the configuration persistence layer and execution dispatcher. This grants full control over daemon scheduling to anyone able to access the TCP port.

## 10. Command Execution
The Runner execution engine cleanly separates executables and arguments via `tokio::process::Command::arg`, neutralizing bash injection for `ExternalApp`s. Shell tasks safely execute via `cmd.exe /c` (Windows) behind the `allow_shell_tasks` safety toggle. However, `Tasker` PowerShell scripts are dynamically generated via `format!()` string interpolation, which crosses the trust boundary and creates PS injection vulnerabilities.

## 11. Filesystem & Path Security
Paths across the application are resolved via `resolve_relative_to_exe_dir`. While this securely anchors relative paths (`./config.json`), it explicitly permits absolute paths (`C:\...`) to bypass the jail. Temporary files (`tempfile::Builder`) are used securely and deleted via `FileCleanupGuard`, preventing persistent predictable-temp-file vulnerabilities.

## 12. CSV/Data Processing
The `crm` binary accurately parses strict base64 downloaded CSVs with `.flexible(false)`. However, `tasker` (`src/tasker/csv_task.rs`) utilizes `.flexible(true)`, flagrantly violating `AGENTS.md` and allowing malformed CSV rows with differing column counts to process silently, risking downstream panics.

## 13. Browser/YasWeb Lifecycle
The headless chrome implementation manages tabs safely but relies on explicit manual `std::thread::sleep` loops (`for i in 0..120`) injected via JavaScript. If a tab crashes, the `tab.evaluate` future returns an error successfully, preventing infinite blocking. However, failing tabs with `--keep-open` hold Rust threads hostage for 60 seconds.

## 14. Scheduler & Pipeline Reliability
The pipeline engine (`src/runner/engine/pipeline.rs`) executes tasks properly, tracking failures and cancelling downstream sequential steps. Parallel steps utilize `tokio::spawn` correctly. The main scheduler loop `start_scheduler` correctly avoids duplicate task execution by checking `running_tasks.iter().any(|t| t.id == task.id)`.

## 15. Async/Concurrency
Concurrency is managed smoothly with Tokio bounded channels. Locks (`Arc<Mutex<RunnerStatus>>`) are obtained and dropped cleanly before `.await` yields. However, multiple blocking I/O calls (`std::fs::remove_file`, `std::fs::create_dir_all`, `walkdir`) occur directly inside Tokio async contexts (e.g., `cleanup_old_logs`), threatening executor thread pool starvation.

## 16. Resource Exhaustion
1. **Unbounded Request Bodies:** The HTTP server (`read_http_request`) reads infinitely if given an enormous `Content-Length`.
2. **Unbounded Channels:** The `ExecutionManagerCommand` channel is bounded to 128 elements, properly protecting memory.
3. **Log Growth:** The daily `cleanup_old_logs` limits log storage securely, mitigating local disk exhaustion.

## 17. Logging & Secret Exposure
Log levels are highly configurable. Secrets (Cognito tokens, Passwords) are explicitly handled via custom `Debug` implementations that emit `***REDACTED***`. However, plain `String` types are used for secrets, so raw memory dumps or accidental future logging additions could expose them.

## 18. Dependency Audit
`cargo-audit` was unavailable; automated advisory scanning was not performed. A manual review of `cargo tree` indicated stable libraries (`tokio v1.52`, `serde`, `reqwest`, `headless_chrome`) with no highly questionable transitive dependencies.

## 19. Previous Recommendation Verification
- **Configuration Decomposition:** Fixed.
- **YasWeb Decomposition:** Fixed.
- **CSV Strictness:** **Regressed**. `flexible(true)` remains in `csv_task.rs`.
- **Pipeline Execution:** Fixed.

## 20. Latest Three PR Review
Verified PR `cab0614` via local git history and `PR_DESCRIPTION.md`. The PR replaced the legacy pipeline model with the new deterministic `TaskStep` sequential/parallel model. No security regressions were identified. The PR maintained proper timeouts and improved overall reliability by cleaning up execution state.

## 21. Test Coverage / Regression Risks
The codebase possesses excellent unit coverage for configuration serialization and scheduling math. However, the custom HTTP server has minimal edge-case test coverage for malformed protocols, and `tasker` lacks isolated testing for PowerShell generation, risking undetected regressions during refactors.

## 22. Remediation Roadmap
1. Limit `buf.len()` in `src/runner/gui/mod.rs` to 1MB.
2. Refactor `cleanup_old_logs` using `tokio::fs` or `spawn_blocking`.
3. Switch `.flexible(true)` to `.flexible(false)` in `src/tasker/csv_task.rs`.
4. Refactor PowerShell scripts to use `-Command` with `-Args` instead of `format!()` string interpolation.
5. Bind `resolve_relative_to_exe_dir` to explicitly deny arbitrary absolute paths.

## 23. Required Regression Tests
- Unbounded TCP Read DoS test (HTTP 1MB cap).
- PowerShell injection payload validation test.
- `csv_task.rs` malformed row rejection unit test (`flexible(false)`).
- Log cleanup starvation test.

## 24. Remaining Technical Debt
The custom TCP HTTP server remains the highest liability. Replacing it with a minimal hardened framework like `axum` or `tiny_http` would resolve all Slowloris, boundary, and path parsing vulnerabilities inherently while keeping the binary size acceptable.
